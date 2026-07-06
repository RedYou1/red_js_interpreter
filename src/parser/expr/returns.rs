use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, LogLevel, Prototype, handle_return, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::{ParseError, Parser},
    },
};

#[derive(Debug, Clone, Copy)]
pub enum ReturnType {
    Return,
    Yield,
    Break,
    YieldBreak,
    Continue,
}

pub struct Return {
    pub rtype: ReturnType,
    pub expr: Option<Box<dyn Expr>>,
}

impl Return {
    pub fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.current().clone();
        parser.bump();
        let t2 = if t == Token::Yield && matches!(parser.current(), Token::Break) {
            parser.bump();
            true
        } else {
            false
        };
        logln(LogLevel::Info, "parse_statement return statement");
        let expr = parser.parse_expression()?;
        if let Token::Semicolon = parser.current() {
            parser.bump();
        }
        Ok(Self {
            expr: Some(expr),
            rtype: match t {
                Token::Break => expr::ReturnType::Break,
                Token::Continue => expr::ReturnType::Continue,
                Token::Return => expr::ReturnType::Return,
                Token::Yield if t2 => expr::ReturnType::YieldBreak,
                Token::Yield => expr::ReturnType::Yield,
                _ => panic!("wierd return"),
            },
        })
    }
}

impl Expr for Return {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let rtype = self.rtype;
        let code: Code = self.expr.compile_expr(mem);
        Box::new(move |_proto, _i| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::Return rtype={:?}", rtype),
            );
            match rtype {
                ReturnType::Return => {
                    let v = handle_return!(code(_proto, _i));
                    logln(
                        LogLevel::Trace,
                        &format!("Exiting Expr::Return Return value={:?}", v),
                    );
                    CodeResult::Return(v)
                }
                ReturnType::Yield => {
                    let v = handle_return!(code(_proto, _i));
                    logln(
                        LogLevel::Trace,
                        &format!("Exiting Expr::Return Yield value={:?}", v),
                    );
                    CodeResult::Yield(v)
                }
                ReturnType::Break => {
                    logln(LogLevel::Trace, "Exiting Expr::Return Break");
                    CodeResult::Break
                }
                ReturnType::YieldBreak => {
                    logln(LogLevel::Trace, "Exiting Expr::Return YieldBreak");
                    CodeResult::YieldBreak
                }
                ReturnType::Continue => {
                    logln(LogLevel::Trace, "Exiting Expr::Return Continue");
                    CodeResult::Continue
                }
            }
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            rtype: self.rtype.clone(),
            expr: self.expr.as_ref().map(|a| a.duplicate_expr()),
        })
    }
}
