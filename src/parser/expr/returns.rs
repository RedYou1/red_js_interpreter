use std::{cell::RefCell, rc::Rc};

use crate::{Code, CodeResult, LogLevel, Prototype, handle_return, logln, parser::expr::Expr};

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
}
