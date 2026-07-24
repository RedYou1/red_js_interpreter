use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, JsValue, LogLevel, Prototype, RUNNABLE, inline_borrow, logln,
    parser::expr::Expr, run_function_object, run_generator_object,
};

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
                JsValue::Generator(_) => Rc::new(RefCell::new(JsValue::Prototype(
                    run_generator_object(func_proto, this, args).into_proto(
                        inline_borrow!(Prototype::find(proto, &stringify!(Generator).into()).1)
                            .unwrap_proto("expr::Call Generator not proto"),
                    ),
                ))),
                _ => {
                    let func = inline_borrow!(func.clone()).unwrap_proto("func not proto");
                    if func.borrow().name.is_some() {
                        let t = func
                            .borrow()
                            .properties
                            .get(&"constructor".into())
                            .expect("call an obj without a constructor")
                            .clone();
                        let func_proto =
                            inline_borrow!(t).unwrap_proto("call an obj without a constructor");
                        match *Prototype::find(func_proto.clone(), &RUNNABLE.into())
                            .1
                            .borrow()
                        {
                            JsValue::Function(_) => run_function_object(func_proto, this, args),
                            JsValue::Generator(_) => Rc::new(RefCell::new(JsValue::Prototype(
                                run_generator_object(func_proto, this, args).into_proto(
                                    inline_borrow!(
                                        Prototype::find(proto, &stringify!(Generator).into()).1
                                    )
                                    .unwrap_proto("expr::Call Generator not proto 2"),
                                ),
                            ))),
                            _ => panic!("call a none function or generator 2 {:?}", func),
                        }
                    } else {
                        panic!("call a none function or generator {:?}", func);
                    }
                }
            };
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Call result={:?}", out),
            );
            CodeResult::Normal(out)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            func: self.func.as_ref().duplicate_expr(),
            args: self
                .args
                .iter()
                .map(|t| t.as_ref().duplicate_expr())
                .collect(),
        })
    }
}
