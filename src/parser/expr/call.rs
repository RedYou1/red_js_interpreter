use std::{cell::RefCell, rc::Rc};

use crate::{Code, CodeResult, JsValue, LogLevel, Prototype, RUNNABLE, logln, new_array, parser::expr::Expr, run_function_object, run_generator_object};

pub struct Call {
    pub func: Box<dyn Expr>,
    pub args: Vec<Box<dyn Expr>>,
}

impl Expr for Call {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let func = self.func.compile_expr(mem.clone());
        let args: Vec<Code> = self
            .args
            .iter()
            .map(|arg| arg.compile_expr(mem.clone()))
            .collect();
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, "Entering Expr::Call");
            let (func, this) = match func(proto.clone(), i) {
                CodeResult::Normal(res) => (res, Rc::new(RefCell::new(JsValue::Undefined))),
                CodeResult::NormalMember(res, of, _) => {
                    (res, Rc::new(RefCell::new(JsValue::Prototype(of))))
                }
                e => return e,
            };
            let func_proto = func.borrow().unwrap_proto("expr::Call func_proto");
            let args = args
                .iter()
                .map(|arg| arg(proto.clone(), i).unwrap_normal())
                .collect();
            let out = match *Prototype::find(func_proto.clone(), &RUNNABLE.into())
                .1
                .borrow()
            {
                JsValue::Function(_) => run_function_object(func_proto, this, args),
                JsValue::Generator(_) => new_array(
                    Prototype::find(proto, &"Array".into())
                        .1
                        .borrow()
                        .unwrap_proto("expr::Call get Array"),
                    run_generator_object(func_proto, this, args).collect(),
                ),
                _ => panic!("call a none function or generator {:?}", func),
            };
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Call result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
}
