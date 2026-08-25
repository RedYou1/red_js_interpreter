use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, LogLevel, Prototype, handle_return, logln,
    parser::{expr::Expr, lexer::Token, parser::Parser},
    run_sub,
};

#[derive(Debug, Clone)]
pub enum ReturnType {
    Return,
    Yield,
    Break(Option<String>),
    YieldBreak,
    Continue(Option<String>),
    Error,
}

#[derive(Debug)]
pub struct Return {
    pub rtype: ReturnType,
    pub expr: Option<Box<dyn Expr>>,
}

impl Return {
    pub fn parse(parser: &mut Parser) -> Self {
        let t = parser.tokens()[parser.index()].clone();
        parser.bump();
        let t2 = if t == Token::Yield && matches!(parser.tokens()[parser.index()], Token::Break) {
            parser.bump();
            true
        } else {
            false
        };
        let name = if matches!(t, Token::Break | Token::Continue) {
            match &parser.tokens()[parser.index()] {
                Token::Ident(name) => Some(name.clone()),
                _ => None,
            }
        } else {
            None
        };
        logln(LogLevel::Info, "Entering Return::parse");
        let expr = Box::new(parser.parse_expression(false));
        if let Token::Semicolon = parser.tokens()[parser.index()] {
            parser.bump();
        }
        Self {
            expr: Some(expr),
            rtype: match t {
                Token::Break => ReturnType::Break(name),
                Token::Continue => ReturnType::Continue(name),
                Token::Return => ReturnType::Return,
                Token::Yield if t2 => ReturnType::YieldBreak,
                Token::Yield => ReturnType::Yield,
                Token::Throw => ReturnType::Error,
                _ => panic!("wierd return"),
            },
        }
    }
}

impl Expr for Return {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let rtype = self.rtype.clone();
        let codes = self.expr.compile(mem);
        vec![Box::new(move |_proto, _i| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::Return rtype={:?}", rtype),
            );
            match rtype.clone() {
                ReturnType::Return => {
                    let v = handle_return!(run_sub(&codes, _proto, &mut CodeIndex::new()));
                    logln(
                        LogLevel::Trace,
                        &format!("Exiting Expr::Return Return value={:?}", v),
                    );
                    CodeResult::Return(v)
                }
                ReturnType::Yield => {
                    let v = handle_return!(run_sub(&codes, _proto, &mut CodeIndex::new()));
                    logln(
                        LogLevel::Trace,
                        &format!("Exiting Expr::Return Yield value={:?}", v),
                    );
                    CodeResult::Yield(v)
                }
                ReturnType::Break(name) => {
                    logln(LogLevel::Trace, "Exiting Expr::Return Break");
                    CodeResult::Break(name)
                }
                ReturnType::YieldBreak => {
                    logln(LogLevel::Trace, "Exiting Expr::Return YieldBreak");
                    CodeResult::YieldBreak
                }
                ReturnType::Continue(name) => {
                    logln(LogLevel::Trace, "Exiting Expr::Return Continue");
                    CodeResult::Continue(name)
                }
                ReturnType::Error => {
                    let v = handle_return!(run_sub(&codes, _proto, &mut CodeIndex::new()));
                    logln(
                        LogLevel::Trace,
                        &format!("Exiting Expr::Return Error value={:?}", v),
                    );
                    CodeResult::Error(v)
                }
            }
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            rtype: self.rtype.clone(),
            expr: self.expr.as_ref().map(|a| a.duplicate()),
        })
    }
}
