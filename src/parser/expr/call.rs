use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, Environment, JsValue, LogLevel, Prototype, RUNNABLE, handle_error,
    inline_borrow, parser::expr::Expr, run_function_object, run_generator_object, run_sub,
};

#[derive(Debug)]
pub struct Call {
    pub func: Box<dyn Expr>,
    pub args: Vec<Box<dyn Expr>>,
}

impl Expr for Call {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let func = self.func.compile(env.clone());
        let args: Vec<Vec<Code>> = self
            .args
            .iter()
            .map(|arg| arg.compile(env.clone()))
            .collect();
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Call");
            let (func, this) = match run_sub(&func, env.clone(), &mut CodeIndex::new()) {
                CodeResult::Normal(res) => (res, Rc::new(RefCell::new(JsValue::Undefined))),
                CodeResult::NormalMember(res, of, _) => {
                    (res, Rc::new(RefCell::new(JsValue::Prototype(of))))
                }
                e => return e,
            };
            let func_proto = func.borrow().unwrap_proto("expr::Call func_proto");
            let mut args_evaluated: Vec<Rc<RefCell<JsValue>>> = Vec::new();
            for arg in args.iter() {
                match run_sub(arg, env.clone(), &mut CodeIndex::new()) {
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
                JsValue::Function(_) => {
                    handle_error!(run_function_object(
                        func_proto,
                        this,
                        args,
                        env.logger.clone()
                    ))
                }
                JsValue::Generator(_) => Rc::new(RefCell::new(JsValue::Prototype(
                    run_generator_object(func_proto, this, args, env.logger.clone()).into_proto(
                        inline_borrow!(
                            Prototype::find(env.mem.clone(), &stringify!(Generator).into()).1
                        )
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
                            .unwrap_or_else(|| {
                                env.logger.borrow_mut().logln_str(
                                    LogLevel::Fatal,
                                    "Expr::Call object has no constructor property",
                                );
                                panic!("call an obj without a constructor")
                            })
                            .clone();
                        let func_proto =
                            inline_borrow!(t).unwrap_proto("call an obj without a constructor");
                        match *Prototype::find(func_proto.clone(), &RUNNABLE.into())
                            .1
                            .borrow()
                        {
                            JsValue::Function(_) => {
                                handle_error!(run_function_object(
                                    func_proto,
                                    this,
                                    args,
                                    env.logger.clone()
                                ))
                            }
                            JsValue::Generator(_) => Rc::new(RefCell::new(JsValue::Prototype(
                                run_generator_object(func_proto, this, args, env.logger.clone())
                                    .into_proto(
                                        inline_borrow!(
                                            Prototype::find(
                                                env.mem.clone(),
                                                &stringify!(Generator).into()
                                            )
                                            .1
                                        )
                                        .unwrap_proto("expr::Call Generator not proto 2"),
                                    ),
                            ))),
                            _ => {
                                env.logger.borrow_mut().logln_str(
                                    LogLevel::Fatal,
                                    "Expr::Call constructor is not a function or generator",
                                );
                                panic!("call a none function or generator 2 {:?}", func)
                            }
                        }
                    } else {
                        env.logger.borrow_mut().logln_str(
                            LogLevel::Fatal,
                            "Expr::Call target is not a function, generator, or constructable object",
                        );
                        panic!("call a none function or generator {:?}", func);
                    }
                }
            };
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!("Exiting Expr::Call result={:?}", out)
            });
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
