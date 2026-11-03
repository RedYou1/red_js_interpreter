use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, handle_return,
    inline_borrow,
    parser::{
        expr::{BinaryOp, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct Assign {
    pub target: Box<dyn Expr>,
    pub value: Box<dyn Expr>,
}

impl Expr for Assign {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let target = self.target.compile(env.clone());
        let value = self.value.compile(env.clone());
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Assign");
            let value = handle_return!(run_sub(&value, env.clone(), &mut CodeIndex::new()));
            let (obj, key) = match run_sub(&target, env.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(key) => {
                    let key_clone = key.clone();
                    let key_val = inline_borrow!(key_clone.clone());
                    if let Some((found_proto, _)) = Prototype::opt_find(env.mem.clone(), &key_val) {
                        (found_proto, key_clone)
                    } else {
                        (env.mem.clone(), key_clone)
                    }
                }
                CodeResult::NormalMember(_, obj, key) => {
                    // Distinguish identifier targets (obj == current proto) from member targets.
                    let key_clone = key.clone();
                    let key_val = inline_borrow!(key_clone.clone());
                    if std::rc::Rc::ptr_eq(&obj, &env.mem) {
                        // Identifier: update nearest binding on the prototype chain.
                        if let Some((found_proto, _)) = Prototype::opt_find(obj.clone(), &key_val) {
                            (found_proto, key_clone)
                        } else {
                            (obj, key_clone)
                        }
                    } else {
                        // Member access: write to the object itself (create/overwrite own property).
                        (obj, key_clone)
                    }
                }
                _ => {
                    env.logger.borrow_mut().logln_str(
                        LogLevel::Error,
                        "Expr::Assign target did not resolve to a member",
                    );
                    panic!("asign not a member")
                }
            };
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Assign {obj:?}[{key:?}] = {value:?}")
            });
            obj.borrow_mut()
                .properties
                .insert(inline_borrow!(key), value.clone());
            CodeResult::Normal(value)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            target: self.target.as_ref().duplicate(),
            value: self.value.as_ref().duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct VarDecl {
    pub name: String,
    pub initializer: Option<Box<dyn Expr>>,
    pub lexical: bool,
}

impl VarDecl {
    pub fn parse(parser: &mut Parser, lexical: bool) -> Self {
        let name = parser.expect_ident();
        parser.env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!("Entering VarDecl::parse name={}", name)
        });
        let initializer: Option<Box<dyn Expr>> =
            if let Token::Assign(t) = &parser.tokens()[parser.index()] {
                assert_eq!(*t, Option::<BinaryOp>::None);
                parser.bump();
                Some(Box::new(parser.parse_expression(false)))
            } else {
                None
            };
        Self {
            name,
            initializer,
            lexical,
        }
    }
}

impl Expr for VarDecl {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let name = self.name.clone();
        let lexical = self.lexical;
        let code = self.initializer.compile(env);
        vec![Box::new(move |env, _| {
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Entering Expr::VarDecl name={}", name)
            });
            let value = handle_return!(run_sub(&code, env.clone(), &mut CodeIndex::new()));
            env.mem
                .borrow_mut()
                .properties
                .insert(name.clone().into(), value.clone());
            if lexical {
                env.mem
                    .borrow_mut()
                    .non_enumerable
                    .insert(name.clone().into());
            }
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::VarDecl name={} value={:?}", name, value)
            });
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            initializer: self.initializer.as_ref().map(|a| a.as_ref().duplicate()),
            lexical: self.lexical,
        })
    }
}

#[derive(Clone, Debug)]
pub enum BindingPattern {
    Identifier(String),
    Array(Vec<Option<BindingPattern>>),
    Object(Vec<(String, BindingPattern)>),
}

impl BindingPattern {
    fn parse(parser: &mut Parser) -> Self {
        match parser.tokens()[parser.index()] {
            Token::Ident(ref name) => {
                let name = name.clone();
                parser.bump();
                Self::Identifier(name)
            }
            Token::LBracket => {
                parser.bump();
                let mut elements = Vec::new();
                while !matches!(
                    parser.tokens()[parser.index()],
                    Token::RBracket | Token::Eof
                ) {
                    if parser.tokens()[parser.index()] == Token::Comma {
                        elements.push(None);
                        parser.bump();
                        continue;
                    }
                    elements.push(Some(Self::parse(parser)));
                    if parser.tokens()[parser.index()] == Token::Comma {
                        parser.bump();
                    } else {
                        break;
                    }
                }
                if parser.tokens()[parser.index()] != Token::RBracket {
                    panic!("expected ']' in binding pattern");
                }
                parser.bump();
                Self::Array(elements)
            }
            Token::LBrace => {
                parser.bump();
                let mut properties = Vec::new();
                while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
                    let is_identifier = matches!(parser.tokens()[parser.index()], Token::Ident(_));
                    let key = match &parser.tokens()[parser.index()] {
                        Token::Ident(key) | Token::Str(key) => key.clone(),
                        Token::Number(key) => key.to_string(),
                        Token::BigInt(key) => key.to_string(),
                        key => key.as_keyword().to_owned(),
                    };
                    parser.bump();
                    let pattern = if parser.tokens()[parser.index()] == Token::Colon {
                        parser.bump();
                        Self::parse(parser)
                    } else if is_identifier {
                        Self::Identifier(key.clone())
                    } else {
                        panic!("expected ':' after binding property key");
                    };
                    properties.push((key, pattern));
                    if parser.tokens()[parser.index()] == Token::Comma {
                        parser.bump();
                    } else {
                        break;
                    }
                }
                if parser.tokens()[parser.index()] != Token::RBrace {
                    panic!("expected '}}' in binding pattern");
                }
                parser.bump();
                Self::Object(properties)
            }
            _ => panic!("expected identifier, array, or object binding pattern"),
        }
    }
}

#[derive(Debug)]
pub struct DestructuringDecl {
    pub pattern: BindingPattern,
    pub initializer: Box<dyn Expr>,
    pub lexical: bool,
}

impl DestructuringDecl {
    pub fn parse(parser: &mut Parser, lexical: bool) -> Self {
        let pattern = BindingPattern::parse(parser);
        if parser.tokens()[parser.index()] != Token::Assign(None) {
            panic!("destructuring declarations require an initializer");
        }
        parser.bump();
        Self {
            pattern,
            initializer: Box::new(parser.parse_expression(false)),
            lexical,
        }
    }
}

impl Expr for DestructuringDecl {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let pattern = self.pattern.clone();
        let lexical = self.lexical;
        let initializer = self.initializer.compile(env);
        vec![Box::new(move |env, _| {
            let value = handle_return!(run_sub(&initializer, env.clone(), &mut CodeIndex::new()));
            match bind_pattern(&env, &pattern, value, lexical) {
                Ok(()) => CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined))),
                Err(error) => error,
            }
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            pattern: self.pattern.clone(),
            initializer: self.initializer.duplicate(),
            lexical: self.lexical,
        })
    }
}

fn bind_pattern(
    env: &Environment,
    pattern: &BindingPattern,
    value: Rc<RefCell<JsValue>>,
    lexical: bool,
) -> Result<(), CodeResult> {
    match pattern {
        BindingPattern::Identifier(name) => {
            env.mem
                .borrow_mut()
                .properties
                .insert(name.clone().into(), value);
            if lexical {
                env.mem
                    .borrow_mut()
                    .non_enumerable
                    .insert(name.clone().into());
            }
        }
        BindingPattern::Array(elements) => {
            let values = crate::prebuild::array::iterable_values(env, value)?;
            for (index, pattern) in elements.iter().enumerate() {
                if let Some(pattern) = pattern {
                    let value = values
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)));
                    bind_pattern(env, pattern, value, lexical)?;
                }
            }
        }
        BindingPattern::Object(properties) => {
            let value = inline_borrow!(value);
            let object_properties = match value {
                JsValue::Prototype(object) => Some(object),
                JsValue::Null | JsValue::Undefined => None,
                JsValue::String(value) => {
                    for (key, pattern) in properties {
                        let property = if key == "length" {
                            Rc::new(RefCell::new(JsValue::BigInt(
                                value.encode_utf16().count() as i64
                            )))
                        } else {
                            key.parse::<usize>()
                                .ok()
                                .and_then(|index| value.chars().nth(index))
                                .map(|character| {
                                    Rc::new(RefCell::new(JsValue::String(character.to_string())))
                                })
                                .unwrap_or_else(|| Rc::new(RefCell::new(JsValue::Undefined)))
                        };
                        bind_pattern(env, pattern, property, lexical)?;
                    }
                    return Ok(());
                }
                _ => {
                    for (key, pattern) in properties {
                        bind_pattern(
                            env,
                            pattern,
                            Rc::new(RefCell::new(JsValue::Undefined)),
                            lexical,
                        )?;
                        let _ = key;
                    }
                    return Ok(());
                }
            };
            let Some(object) = object_properties else {
                return Err(binding_type_error(env));
            };
            for (key, pattern) in properties {
                let property = Prototype::find(object.clone(), &key.clone().into()).1;
                bind_pattern(env, pattern, property, lexical)?;
            }
        }
    }
    Ok(())
}

fn binding_type_error(env: &Environment) -> CodeResult {
    let prototype = Prototype::find(env.mem.clone(), &stringify!(TypeError).into())
        .1
        .borrow()
        .unwrap_proto("Destructuring TypeError");
    let constructor = Rc::new(RefCell::new(JsValue::Prototype(prototype.clone())));
    CodeResult::Error(Rc::new(RefCell::new(JsValue::Prototype(
        Prototype::new_child(prototype, None, [("constructor".into(), constructor)]),
    ))))
}
