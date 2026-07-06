use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, LogLevel, Prototype, handle_return, inline_borrow, logln, parser::expr::Expr,
};

pub struct Member {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for Member {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let obj = self.object.compile_expr(mem.clone());
        let prop = self.property.compile_expr(mem);
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, "Entering Expr::Member");
            let obj = handle_return!(obj(proto.clone(), i))
                .borrow()
                .unwrap_proto("expr::Member for obj");
            let key = handle_return!(prop(proto, i));
            let out = Prototype::find(obj.clone(), &inline_borrow!(key.clone())).1;
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Member {obj:?}[{key:?}] == {out:?}"),
            );
            CodeResult::NormalMember(out, obj, key)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate_expr(),
            property: self.property.duplicate_expr(),
        })
    }
}
