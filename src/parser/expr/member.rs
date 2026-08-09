use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::expr::Expr, run_sub,
};

#[derive(Debug)]
pub struct Member {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for Member {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let obj = self.object.compile(mem.clone());
        let prop = self.property.compile(mem);
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::Member");
            let obj = handle_return!(run_sub(&obj, proto.clone(), &mut CodeIndex::new()))
                .borrow()
                .unwrap_proto("expr::Member for obj");
            let key = handle_return!(run_sub(&prop, proto, &mut CodeIndex::new()));
            let out = Prototype::find(obj.clone(), &inline_borrow!(key.clone())).1;
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Member {obj:?}[{key:?}] == {out:?}"),
            );
            CodeResult::NormalMember(out, obj, key)
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            object: self.object.duplicate(),
            property: self.property.duplicate(),
        })
    }
}
