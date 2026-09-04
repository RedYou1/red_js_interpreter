use crate::{
    Code, CodeIndex, CodeResult, Environment, LogLevel, Prototype, handle_return, inline_borrow,
    parser::expr::Expr, run_sub,
};

#[derive(Debug)]
pub struct Member {
    pub object: Box<dyn Expr>,
    pub property: Box<dyn Expr>,
}

impl Expr for Member {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let obj = self.object.compile(env.clone());
        let prop = self.property.compile(env);
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Member");
            let obj = handle_return!(run_sub(&obj, env.clone(), &mut CodeIndex::new()))
                .borrow()
                .unwrap_proto("expr::Member for obj");
            let key = handle_return!(run_sub(&prop, env.clone(), &mut CodeIndex::new()));
            let out = Prototype::find(obj.clone(), &inline_borrow!(key.clone())).1;
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Member {obj:?}[{key:?}] == {out:?}")
            });
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
