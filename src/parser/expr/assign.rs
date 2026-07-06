use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, LogLevel, Prototype, handle_return, inline_borrow, logln, parser::expr::Expr,
};

pub struct Assign {
    pub target: Box<dyn Expr>,
    pub value: Box<dyn Expr>,
}

impl Expr for Assign {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let target = self.target.compile_expr(mem.clone());
        let value = self.value.compile_expr(mem.clone());
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, "Entering Expr::Assign");
            let value = handle_return!(value(proto.clone(), i));
            let (obj, key) = match target(proto.clone(), i) {
                CodeResult::Normal(key) => (proto, key),
                CodeResult::NormalMember(_, obj, key) => (obj, key),
                _ => panic!("asign not a member"),
            };
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Assign {obj:?}[{key:?}] = {value:?}"),
            );
            obj.borrow_mut()
                .properties
                .insert(inline_borrow!(key), value.clone());
            CodeResult::Normal(value)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            target: self.target.as_ref().duplicate_expr(),
            value: self.value.as_ref().duplicate_expr(),
        })
    }
}
