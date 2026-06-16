use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, PROTOTYPE_NAME, Prototype, RUNNABLE, handle_return,
    inline_borrow, logln, parser::expr::Expr, run_function_object,
};

pub struct New {
    pub constructor: Box<dyn Expr>,
    pub args: Vec<Box<dyn Expr>>,
}

impl Expr for New {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let constructor = self.constructor.compile_expr(mem.clone());
        let args: Vec<Code> = self
            .args
            .iter()
            .map(|arg| arg.compile_expr(mem.clone()))
            .collect();
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, "Entering Expr::New");
            let mut class = handle_return!(constructor(proto.clone(), i))
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
            let out = run_function_object(
                constructor,
                Rc::new(RefCell::new(JsValue::Prototype(new_obj.clone()))),
                args.iter()
                    .map(|arg| arg(proto.clone(), i).unwrap_normal())
                    .collect(),
            );
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::New new_obj={new_obj:?} out={out:?}"),
            );
            // In JavaScript, if the constructor returns an object, that object is used
            // Otherwise, the newly created object is used
            let result = match inline_borrow!(out.clone()) {
                JsValue::Prototype(_) => out,
                _ => Rc::new(RefCell::new(JsValue::Prototype(new_obj))),
            };
            CodeResult::Normal(result)
        })
    }
}
