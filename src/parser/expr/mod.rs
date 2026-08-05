use std::{cell::RefCell, fmt::Debug, mem::MaybeUninit, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::stmt::Stmt, run_sub,
};

pub trait Expr: Stmt + Debug {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code;
    fn duplicate_expr(&self) -> Box<dyn Expr>;
}

impl<E: Expr> Stmt for E {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        vec![self.compile_expr(mem)]
    }
    fn duplicate_stmt(&self) -> Box<dyn Stmt> {
        self.duplicate_expr()
    }
}

impl Expr for Option<Box<dyn Expr>> {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        if let Some(e) = self {
            e.compile_expr(mem)
        } else {
            Box::new(|_, _| CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined))))
        }
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(self.as_ref().map(|a| Expr::duplicate_expr(a.as_ref())))
    }
}

impl<const LEN: usize> Expr for [Box<dyn Expr>; LEN] {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let codes: Vec<Code> = self
            .iter()
            .map(|code| code.compile_expr(mem.clone()))
            .collect();
        Box::new(move |proto, _| run_sub(codes.as_ref(), proto, &mut CodeIndex::new()))
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        let mut t = [const { MaybeUninit::<Box<dyn Expr>>::uninit() }; LEN];
        for (v, t) in self.iter().zip(t.iter_mut()) {
            t.write(v.as_ref().duplicate_expr());
        }
        Box::new(unsafe { MaybeUninit::array_assume_init(t) })
    }
}

impl<const LEN: usize> Stmt for [Box<dyn Stmt>; LEN] {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let codes: Vec<Code> = self
            .iter()
            .flat_map(|code| code.compile_stmt(mem.clone()))
            .collect();
        vec![Box::new(move |proto, _| {
            run_sub(codes.as_ref(), proto, &mut CodeIndex::new())
        })]
    }
    fn duplicate_stmt(&self) -> Box<dyn Stmt> {
        let mut t = [const { MaybeUninit::<Box<dyn Stmt>>::uninit() }; LEN];
        for (v, t) in self.iter().zip(t.iter_mut()) {
            t.write(v.as_ref().duplicate_stmt());
        }
        Box::new(unsafe { MaybeUninit::array_assume_init(t) })
    }
}

mod assign;
mod call;
mod class_decl;
mod consts;
mod function_decl;
mod member;
mod new;
mod object;
mod operators;
mod postfix;
mod returns;
mod template_str;

pub use {
    assign::*, call::*, class_decl::*, consts::*, function_decl::*, member::*, new::*, object::*,
    operators::*, postfix::*, returns::*, template_str::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
}

impl Expr for Identifier {
    fn compile_expr(&self, _: Rc<RefCell<Prototype>>) -> Code {
        let name = self.name.clone();
        Box::new(move |proto, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::Identifier name={}", name),
            );
            let res = Prototype::find(proto.clone(), &name.as_str().into()).1;
            if name == "super" {
                let this = Prototype::find(proto, &"this".into()).1;
                logln(
                    LogLevel::Trace,
                    &format!("Exiting Expr::Identifier result={:?} of {:?}", res, this),
                );
                CodeResult::NormalMember(
                    res,
                    inline_borrow!(this)
                        .unwrap_proto("expr::Identifier this is suposed to be proto"),
                    Rc::new(RefCell::new("this".into())),
                )
            } else {
                logln(
                    LogLevel::Trace,
                    &format!("Exiting Expr::Identifier result={:?}", res),
                );
                CodeResult::NormalMember(res, proto, Rc::new(RefCell::new(name.as_str().into())))
            }
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct Typeof {
    pub obj: Box<dyn Stmt>,
}

impl Expr for Typeof {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let obj = self.obj.compile_stmt(mem);
        Box::new(move |prop, _| {
            if obj.len() > 1 {
                handle_return!(run_sub(
                    &obj[..(obj.len() - 1)],
                    prop.clone(),
                    &mut CodeIndex::new()
                ));
            }
            let t = handle_return!(obj[obj.len() - 1](prop, &mut CodeIndex::new()));
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::String(
                match inline_borrow!(t) {
                    JsValue::Function(_) => "function",
                    JsValue::Generator(_) => "function",
                    JsValue::Prototype(proto) => {
                        if proto.borrow().name.is_some() {
                            "function"
                        } else {
                            "object"
                        }
                    }
                    JsValue::Symbol(_, _) => "symbol",
                    JsValue::String(_) => "string",
                    JsValue::Number(_) | JsValue::BigInt(_) => "number",
                    JsValue::Boolean(_) => "boolean",
                    JsValue::Undefined => "undefined",
                    JsValue::Null => "object",
                }
                .to_owned(),
            ))))
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            obj: self.obj.duplicate_stmt(),
        })
    }
}

#[derive(Debug)]
pub struct Label {
    pub name: String,
    pub code: Vec<Box<dyn Stmt>>,
}

impl Expr for Label {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let codes = self.code.compile_stmt(mem);
        let wanted_name = self.name.clone();
        Box::new(
            move |proto, _| match run_sub(&codes, proto, &mut CodeIndex::new()) {
                CodeResult::Break(None) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                CodeResult::Break(Some(name)) if wanted_name.eq(name.as_str()) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                CodeResult::Continue(None) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                CodeResult::Continue(Some(name)) if wanted_name.eq(name.as_str()) => {
                    CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
                }
                a => a,
            },
        )
    }

    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            code: self.code.iter().map(|a| a.duplicate_stmt()).collect(),
        })
    }
}
