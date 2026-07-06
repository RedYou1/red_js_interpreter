use std::{cell::RefCell, fmt::Debug, mem::MaybeUninit, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        lexer::Token,
        parser::{ParseError, Parser},
        stmt::Stmt,
    },
    run_sub,
};

pub trait Expr: Stmt {
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
                CodeResult::Normal(res)
            }
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(self.clone())
    }
}

pub struct VarDecl {
    pub name: String,
    pub initializer: Option<Box<dyn Expr>>,
}

impl VarDecl {
    pub fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let name = parser.expect_ident()?;
        logln(
            LogLevel::Info,
            &format!("parse_statement variable declaration name={}", name),
        );
        let initializer = if let Token::Assign(t) = parser.current() {
            assert_eq!(*t, Option::<BinaryOp>::None);
            parser.bump();
            Some(parser.parse_expression()?)
        } else {
            None
        };
        if let Token::Semicolon = parser.current() {
            parser.bump();
        }
        Ok(Self { name, initializer })
    }
}

impl Expr for VarDecl {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let name = self.name.clone();
        let code = self.initializer.compile_expr(mem);
        Box::new(move |proto, _| {
            logln(
                LogLevel::Trace,
                &format!("Entering Expr::VarDecl name={}", name),
            );
            let value = handle_return!(code(proto.clone(), &mut CodeIndex::new()));
            proto
                .borrow_mut()
                .properties
                .insert(name.clone().into(), value.clone());
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::VarDecl name={} value={:?}", name, value),
            );
            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name.clone(),
            initializer: self
                .initializer
                .as_ref()
                .map(|a| a.as_ref().duplicate_expr()),
        })
    }
}
