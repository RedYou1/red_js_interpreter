use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, RUNNABLE, handle_error,
    inline_borrow, logln, parser::expr::Expr, run_function_object, run_generator_object, run_sub,
};

#[derive(Debug)]
pub struct Call {
    pub func: Box<dyn Expr>,
    pub args: Vec<Box<dyn Expr>>,
}

impl Expr for Call {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let func = self.func.compile(mem.clone());
        let args: Vec<Vec<Code>> = self
            .args
            .iter()
            .map(|arg| arg.compile(mem.clone()))
            .collect();
        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::Call");
            let (func, this) = match run_sub(&func, proto.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(res) => (res, Rc::new(RefCell::new(JsValue::Undefined))),
                CodeResult::NormalMember(res, of, _) => {
                    (res, Rc::new(RefCell::new(JsValue::Prototype(of))))
                }
                e => return e,
            };
            let func_proto = func.borrow().unwrap_proto("expr::Call func_proto");
            let mut args_evaluated: Vec<Rc<RefCell<JsValue>>> = Vec::new();
            for arg in args.iter() {
                match run_sub(arg, proto.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(res) => args_evaluated.push(res),
                    CodeResult::NormalMember(res, _, _) => args_evaluated.push(res),
                    e => return e,
                }
            }
            let args = args_evaluated;
            let out = match *Prototype::find(func_proto.clone(), &RUNNABLE.into())
                .1
                .borrow()
            {
                JsValue::Function(_) => handle_error!(run_function_object(func_proto, this, args)),
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
                            JsValue::Function(_) => {
                                handle_error!(run_function_object(func_proto, this, args))
                            }
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
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            func: self.func.as_ref().duplicate(),
            args: self.args.iter().map(|t| t.as_ref().duplicate()).collect(),
        })
    }
}
