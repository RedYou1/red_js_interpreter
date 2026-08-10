use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, PROTOTYPE_NAME, Prototype, RUNNABLE,
    handle_error, handle_return, inline_borrow, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_function_object, run_sub,
};

#[derive(Debug)]
pub struct New {
    pub constructor: Box<dyn Expr>,
    pub args: Vec<Box<dyn Expr>>,
}

impl New {
    pub fn parse(parser: &mut Parser) -> Self {
        let mut constructor = parser.parse_primary();
        loop {
            if parser.tokens()[parser.index()] == Token::Dot {
                parser.bump();
                if let Token::Ident(name) = &parser.tokens()[parser.index()] {
                    let prop = name.clone();
                    parser.bump();
                    constructor = Box::new(expr::Member {
                        object: constructor,
                        property: Box::new(expr::ConstString { s: prop }),
                    });
                    continue;
                } else {
                    panic!(
                        "expected property name after '.', got {:?}",
                        parser.tokens()[parser.index()]
                    );
                }
            }
            if parser.tokens()[parser.index()] == Token::LBracket {
                parser.bump();
                let index = parser.parse_expression();
                if parser.tokens()[parser.index()] != Token::RBracket {
                    panic!("expected ']'");
                }
                parser.bump();
                constructor = Box::new(expr::Member {
                    object: constructor,
                    property: index,
                });
                continue;
            }
            break;
        }
        let mut args = Vec::new();
        if parser.tokens()[parser.index()] == Token::LParen {
            parser.bump();
            while !matches!(parser.tokens()[parser.index()], Token::RParen | Token::Eof) {
                args.push(parser.parse_expression());
                if parser.tokens()[parser.index()] == Token::Comma {
                    parser.bump();
                }
            }
            if parser.tokens()[parser.index()] == Token::RParen {
                parser.bump();
            }
        }
        Self { constructor, args }
    }
}

impl Expr for New {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let constructor = self.constructor.compile(mem.clone());
        let args: Vec<Vec<Code>> = self
            .args
            .iter()
            .map(|arg| arg.compile(mem.clone()))
            .collect();
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::New");
            let mut class =
                handle_return!(run_sub(&constructor, proto.clone(), &mut CodeIndex::new()))
                    .borrow()
                    .unwrap_proto("expr::New for constructor");
            let constructor = if Prototype::opt_find(class.clone(), &RUNNABLE.into()).is_some() {
                class = inline_borrow!(Prototype::find(class.clone(), &PROTOTYPE_NAME.into()).1)
                    .unwrap_proto("expr::New get prototype");

                inline_borrow!(Prototype::find(class.clone(), &"constructor".into()).1)
                    .unwrap_proto("expr::New get prototype constructor")
            } else {
                inline_borrow!(
                    Prototype::find(class.clone(), &JsValue::String("constructor".to_owned())).1
                )
                .unwrap_proto("expr::New get constructor in class")
            };
            let new_obj = Prototype::new_child(class.clone(), None, []);
            let mut args_evaluated: Vec<Rc<RefCell<JsValue>>> = Vec::new();
            for arg in args.iter() {
                match run_sub(arg, proto.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => args_evaluated.push(res),
                    CodeResult::NormalMember(res, _, _) => args_evaluated.push(res),
                    e => return e,
                }
            }
            let out = run_function_object(
                constructor,
                Rc::new(RefCell::new(JsValue::Prototype(new_obj.clone()))),
                args_evaluated,
            );
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::New new_obj={new_obj:?} out={out:?}"),
            );
            let out = handle_error!(out);
            // In JavaScript, if the constructor returns an object, that object is used
            // Otherwise, the newly created object is used
            let result = match inline_borrow!(out.clone()) {
                JsValue::Prototype(_) => out,
                _ => Rc::new(RefCell::new(JsValue::Prototype(new_obj))),
            };
            CodeResult::Normal(result)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            constructor: self.constructor.duplicate(),
            args: self.args.iter().map(|t| t.duplicate()).collect(),
        })
    }
}
