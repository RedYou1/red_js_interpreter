use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln,
    parser::expr::Expr, run_sub,
};

#[derive(Debug)]
pub struct Postfix {
    pub expr: Box<dyn Expr>,
    pub inc: bool,
}

impl Expr for Postfix {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let inc = self.inc;
        let target = self.expr.compile(mem.clone());
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, &format!("Entering Expr::Postfix op:{inc}"));
            let target_ref = handle_return!(run_sub(&target, proto.clone(), &mut CodeIndex::new()));
            let old_value = target_ref.borrow().clone();
            let new_value = match &old_value {
                JsValue::Number(n) => JsValue::Number(*n + if inc { 1.0 } else { -1.0 }),
                JsValue::BigInt(n) => JsValue::BigInt(*n + if inc { 1 } else { -1 }),
                _ => JsValue::Undefined,
            };
            *target_ref.borrow_mut() = new_value.clone();
            let out = Rc::new(RefCell::new(old_value));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Postfix result={:?}", out),
            );
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            expr: self.expr.duplicate(),
            inc: self.inc,
        })
    }
}

#[derive(Debug)]
pub struct Prefix {
    pub expr: Box<dyn Expr>,
    pub inc: bool,
}

impl Expr for Prefix {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let inc = self.inc;
        let target = self.expr.compile(mem.clone());
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, &format!("Entering Expr::Prefix op:{inc}"));
            let target_ref = handle_return!(run_sub(&target, proto.clone(), &mut CodeIndex::new()));
            let old_value = target_ref.borrow().clone();
            let new_value = match &old_value {
                JsValue::Number(n) => JsValue::Number(*n + if inc { 1.0 } else { -1.0 }),
                JsValue::BigInt(n) => JsValue::BigInt(*n + if inc { 1 } else { -1 }),
                _ => JsValue::Undefined,
            };
            *target_ref.borrow_mut() = new_value.clone();
            let out = Rc::new(RefCell::new(new_value));
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Prefix result={:?}", out),
            );
            CodeResult::Normal(out)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            expr: self.expr.duplicate(),
            inc: self.inc,
        })
    }
}
