use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, inline_borrow,
    new_array,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub enum ObjectEntry {
    Property(Box<dyn Expr>, Box<dyn Expr>),
    Spread(Box<dyn Expr>),
}

#[derive(Debug)]
pub struct Object {
    pub entries: Vec<ObjectEntry>,
}

impl Object {
    pub fn parse(parser: &mut Parser) -> Self {
        parser.bump();
        let mut entries = Vec::new();
        while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
            if parser.tokens()[parser.index()] == Token::Spread {
                parser.bump();
                entries.push(ObjectEntry::Spread(Box::new(
                    parser.parse_expression(false),
                )));
                if parser.tokens()[parser.index()] == Token::Comma {
                    parser.bump();
                }
                continue;
            }
            let start = if let Token::Star = parser.tokens()[parser.index()] {
                parser.bump();
                true
            } else {
                false
            };
            let key: Box<dyn Expr> = if let Token::Ident(value) = &parser.tokens()[parser.index()] {
                let key = Box::new(expr::ConstString { s: value.clone() });
                parser.bump();
                key
            } else {
                //false
                parser.parse_primary()
            };
            match parser.tokens()[parser.index()] {
                Token::Colon => {
                    parser.bump();
                    let value = Box::new(parser.parse_expression(false));
                    entries.push(ObjectEntry::Property(key, value));
                }
                Token::LParen => {
                    let mut func = expr::FunctionDecl::parse(parser, false);
                    func.generator = start;
                    entries.push(ObjectEntry::Property(key, Box::new(func)));
                }
                _ => {
                    parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                        format!(
                            "Object::parse expected ':' or '(' at index {} but found {:?}",
                            parser.index(),
                            parser.tokens()[parser.index()]
                        )
                    });
                    panic!(
                        "expected ':' or '(' in object literal, got {:?}",
                        parser.tokens()[parser.index()]
                    )
                }
            }
            if let Token::Comma = parser.tokens()[parser.index()] {
                parser.bump();
            }
        }
        if parser.tokens()[parser.index()] != Token::RBrace {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "Object::parse expected '}}' at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
            panic!("expected '}}' at end of object literal");
        }
        parser.bump();
        Self { entries }
    }
}

impl Expr for Object {
    fn compile(&self, env: Environment) -> Vec<Code> {
        enum CompiledEntry {
            Property(Vec<Code>, Vec<Code>),
            Spread(Vec<Code>),
        }
        let elems: Vec<CompiledEntry> = self
            .entries
            .iter()
            .map(|entry| match entry {
                ObjectEntry::Property(key, value) => {
                    CompiledEntry::Property(key.compile(env.clone()), value.compile(env.clone()))
                }
                ObjectEntry::Spread(value) => CompiledEntry::Spread(value.compile(env.clone())),
            })
            .collect();
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Object");
            let object_proto = Prototype::find(env.mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("expr::Object for Object");
            let object = Prototype::new_child(object_proto, None, []);
            for entry in elems.iter() {
                match entry {
                    CompiledEntry::Property(key, value) => {
                        let key_val = match run_sub(key, env.clone(), &mut CodeIndex::new()) {
                            CodeResult::Normal(res) | CodeResult::NormalMember(res, _, _) => {
                                inline_borrow!(res)
                            }
                            e => return e,
                        };
                        let value_val = match run_sub(value, env.clone(), &mut CodeIndex::new()) {
                            CodeResult::Normal(res) | CodeResult::NormalMember(res, _, _) => res,
                            e => return e,
                        };
                        let mut object = object.borrow_mut();
                        object.properties.insert(key_val.clone(), value_val);
                        object.non_enumerable.remove(&key_val);
                    }
                    CompiledEntry::Spread(value) => {
                        let source = match run_sub(value, env.clone(), &mut CodeIndex::new()) {
                            CodeResult::Normal(res) | CodeResult::NormalMember(res, _, _) => res,
                            e => return e,
                        };
                        copy_enumerable_properties(&source, &object);
                    }
                }
            }
            let out = Rc::new(RefCell::new(JsValue::Prototype(object)));
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Object result={:?}", out)
            });
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            entries: self
                .entries
                .iter()
                .map(|entry| match entry {
                    ObjectEntry::Property(key, value) => {
                        ObjectEntry::Property(key.duplicate(), value.duplicate())
                    }
                    ObjectEntry::Spread(value) => ObjectEntry::Spread(value.duplicate()),
                })
                .collect(),
        })
    }
}

fn copy_enumerable_properties(source: &Rc<RefCell<JsValue>>, target: &Rc<RefCell<Prototype>>) {
    match inline_borrow!(source.clone()) {
        JsValue::Prototype(source) => {
            let properties = {
                let source = source.borrow();
                source
                    .properties
                    .iter()
                    .filter(|(key, _)| {
                        **key != JsValue::String("__proto__".to_owned())
                            && !source.non_enumerable.contains(*key)
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<Vec<_>>()
            };
            let mut target = target.borrow_mut();
            for (key, value) in properties {
                target.properties.insert(key.clone(), value);
                target.non_enumerable.remove(&key);
            }
        }
        JsValue::String(value) => {
            let mut target = target.borrow_mut();
            for (index, character) in value.chars().enumerate() {
                target.properties.insert(
                    index.into(),
                    Rc::new(RefCell::new(JsValue::String(character.to_string()))),
                );
            }
        }
        JsValue::Null
        | JsValue::Undefined
        | JsValue::Boolean(_)
        | JsValue::Number(_)
        | JsValue::BigInt(_) => {}
        JsValue::Symbol(_, _)
        | JsValue::Function(_)
        | JsValue::Generator(_)
        | JsValue::RedGex(_) => {}
    }
}

#[derive(Debug)]
pub enum ArrayElement {
    Value(Box<dyn Expr>),
    Spread(Box<dyn Expr>),
}

#[derive(Debug)]
pub struct Array {
    pub elems: Vec<ArrayElement>,
}

impl Expr for Array {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let elems: Vec<(Vec<Code>, bool)> = self
            .elems
            .iter()
            .map(|elem| match elem {
                ArrayElement::Value(elem) => (elem.compile(env.clone()), false),
                ArrayElement::Spread(elem) => (elem.compile(env.clone()), true),
            })
            .collect();
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Array");
            let array_proto = Prototype::find(env.mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("expr::Array for Array");
            let mut values: Vec<Rc<RefCell<JsValue>>> = Vec::new();
            for (elem, spread) in elems.iter() {
                let value = match run_sub(elem, env.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) | CodeResult::NormalMember(res, _, _) => res,
                    e => return e,
                };
                if *spread {
                    match crate::prebuild::array::iterable_values(&env, value) {
                        Ok(elements) => values.extend(elements),
                        Err(error) => return error,
                    }
                } else {
                    values.push(value);
                }
            }
            let out = new_array(array_proto, values, env.logger.clone());
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Array result={:?}", out)
            });
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            elems: self
                .elems
                .iter()
                .map(|element| match element {
                    ArrayElement::Value(value) => ArrayElement::Value(value.duplicate()),
                    ArrayElement::Spread(value) => ArrayElement::Spread(value.duplicate()),
                })
                .collect(),
        })
    }
}
