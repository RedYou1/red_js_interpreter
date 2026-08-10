use std::{cell::RefCell, fmt::Debug, mem::MaybeUninit, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{lexer::Token, parser::Parser},
    run_sub,
};

pub trait Expr: Debug {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code>;
    fn duplicate(&self) -> Box<dyn Expr>;
}

impl Expr for Option<Box<dyn Expr>> {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        if let Some(e) = self {
            e.compile(mem)
        } else {
            vec![Box::new(|_, _| {
                CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
            })]
        }
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.as_ref().map(|a| Expr::duplicate(a.as_ref())))
    }
}

impl<const LEN: usize> Expr for [Box<dyn Expr>; LEN] {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        self.iter()
            .flat_map(|code| code.compile(mem.clone()))
            .collect()
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        let mut t = [const { MaybeUninit::<Box<dyn Expr>>::uninit() }; LEN];
        for (v, t) in self.iter().zip(t.iter_mut()) {
            t.write(v.as_ref().duplicate());
        }
        Box::new(unsafe { MaybeUninit::array_assume_init(t) })
    }
}

impl Expr for Vec<Box<dyn Expr>> {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        self.iter()
            .flat_map(|stmt| stmt.compile(mem.clone()))
            .collect()
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(
            self.iter()
                .map(|a| a.as_ref().duplicate())
                .collect::<Self>(),
        )
    }
}

mod assign;
mod call;
mod class_decl;
mod consts;
mod function_decl;
mod if_decl;
mod loop_decl;
mod member;
mod new;
mod object;
mod operators;
mod postfix;
mod returns;
mod template_str;
mod try_catch;

pub use {
    assign::*, call::*, class_decl::*, consts::*, function_decl::*, if_decl::*, loop_decl::*,
    member::*, new::*, object::*, operators::*, postfix::*, returns::*, template_str::*,
    try_catch::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
}

impl Expr for Identifier {
    fn compile(&self, _: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let name = self.name.clone();
        vec![Box::new(move |proto, _| {
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
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct Typeof {
    pub obj: Box<dyn Expr>,
}

impl Typeof {
    pub fn parse(parser: &mut Parser) -> Self {
        let t = if let Token::LParen = parser.tokens()[parser.index()] {
            parser.bump();
            true
        } else {
            false
        };
        let expr = parser.parse_statement().unwrap();
        if t {
            if let Token::RParen = parser.tokens()[parser.index()] {
                parser.bump();
            } else {
                panic!(
                    "Typeof Englobe paren in parse_primary {:?} {:?} {:?}",
                    parser.tokens()[parser.index()],
                    parser.tokens()[parser.index() + 1],
                    parser.tokens()[parser.index() + 2]
                );
            }
        }
        Self { obj: expr }
    }
}

impl Expr for Typeof {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let obj = self.obj.compile(mem);
        vec![Box::new(move |prop, _| {
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
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            obj: self.obj.duplicate(),
        })
    }
}

#[derive(Debug)]
pub struct Label {
    pub name: String,
    pub code: Vec<Box<dyn Expr>>,
}

impl Expr for Label {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let codes = self.code.compile(mem);
        let wanted_name = self.name.clone();
        vec![Box::new(move |proto, _| {
            match run_sub(&codes, proto, &mut CodeIndex::new()) {
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
            }
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            code: self.code.iter().map(|a| a.duplicate()).collect(),
        })
    }
}
