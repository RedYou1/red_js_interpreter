use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, inline_borrow, logln, new_array,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct Object {
    pub properties: Vec<(Box<dyn Expr>, Box<dyn Expr>)>,
}

impl Object {
    pub fn parse(parser: &mut Parser) -> Self {
        parser.bump();
        let mut properties = Vec::new();
        while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
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
                parser.parse_primary()
            };
            match parser.tokens()[parser.index()] {
                Token::Colon => {
                    parser.bump();
                    let value = parser.parse_expression();
                    properties.push((key, value));
                }
                Token::LParen => {
                    let mut func = expr::FunctionDecl::parse(parser, false);
                    func.generator = start;
                    properties.push((key, Box::new(func)));
                }
                _ => panic!(
                    "expected ':' or '(' in object literal, got {:?}",
                    parser.tokens()[parser.index()]
                ),
            }
            if let Token::Comma = parser.tokens()[parser.index()] {
                parser.bump();
            }
        }
        if parser.tokens()[parser.index()] != Token::RBrace {
            panic!("expected '}}' at end of object literal");
        }
        parser.bump();
        Self { properties }
    }
}

impl Expr for Object {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let elems: Vec<(Vec<Code>, Vec<Code>)> = self
            .properties
            .iter()
            .map(|(key, value)| (key.compile(mem.clone()), value.compile(mem.clone())))
            .collect();
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::Object");
            let object_proto = Prototype::find(mem.clone(), &stringify!(Object).into())
                .1
                .borrow()
                .unwrap_proto("expr::Object for Object");
            let mut props: Vec<(JsValue, Rc<RefCell<JsValue>>)> = Vec::new();
            for (key, value) in elems.iter() {
                let key_val = match run_sub(key, proto.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => inline_borrow!(res),
                    CodeResult::NormalMember(res, _, _) => inline_borrow!(res),
                    e => return e,
                };
                let value_val = match run_sub(value, proto.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => res,
                    CodeResult::NormalMember(res, _, _) => res,
                    e => return e,
                };
                props.push((key_val, value_val));
            }
            let out = Rc::new(RefCell::new(JsValue::Prototype(Prototype::new_child(
                object_proto,
                None,
                props,
            ))));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Object result={:?}", out),
            );
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            properties: self
                .properties
                .iter()
                .map(|(a, b)| (a.duplicate(), b.duplicate()))
                .collect(),
        })
    }
}

#[derive(Debug)]
pub struct Array {
    pub elems: Vec<Box<dyn Expr>>,
}

impl Expr for Array {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let elems: Vec<Vec<Code>> = self
            .elems
            .iter()
            .map(|elem| elem.compile(mem.clone()))
            .collect();
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::Array");
            let array_proto = Prototype::find(mem.clone(), &stringify!(Array).into())
                .1
                .borrow()
                .unwrap_proto("expr::Array for Array");
            let mut values: Vec<Rc<RefCell<JsValue>>> = Vec::new();
            for elem in elems.iter() {
                match run_sub(elem, proto.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => values.push(res),
                    CodeResult::NormalMember(res, _, _) => values.push(res),
                    e => return e,
                }
            }
            let out = new_array(array_proto, values);
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Array result={:?}", out),
            );
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            elems: self.elems.iter().map(|a| a.duplicate()).collect(),
        })
    }
}
