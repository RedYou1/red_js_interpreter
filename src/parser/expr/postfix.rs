use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, handle_return, logln, parser::expr::Expr,
};

pub struct Postfix {
    pub expr: Box<dyn Expr>,
    pub inc: bool,
}

impl Expr for Postfix {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let inc = self.inc;
        let target = self.expr.compile_expr(mem.clone());
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, &format!("Entering Expr::Postfix op:{inc}"));
            let target_ref = handle_return!(target(proto.clone(), i));
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
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            expr: self.expr.duplicate_expr(),
            inc: self.inc,
        })
    }
}
