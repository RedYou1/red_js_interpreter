use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln, parser::{expr::{BinaryOp, Expr}, lexer::Token, parser::Parser},
};

pub struct Assign {
    pub target: Box<dyn Expr>,
    pub value: Box<dyn Expr>,
}

impl Expr for Assign {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let target = self.target.compile_expr(mem.clone());
        let value = self.value.compile_expr(mem.clone());
        Box::new(move |proto, i| {
            logln(LogLevel::Trace, "Entering Expr::Assign");
            let value = handle_return!(value(proto.clone(), i));
            let (obj, key) = match target(proto.clone(), i) {
                CodeResult::Normal(key) => (proto, key),
                CodeResult::NormalMember(_, obj, key) => (obj, key),
                _ => panic!("asign not a member"),
            };
            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::Assign {obj:?}[{key:?}] = {value:?}"),
            );
            obj.borrow_mut()
                .properties
                .insert(inline_borrow!(key), value.clone());
            CodeResult::Normal(value)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            target: self.target.as_ref().duplicate_expr(),
            value: self.value.as_ref().duplicate_expr(),
        })
    }
}

pub struct VarDecl {
    pub name: String,
    pub initializer: Option<Box<dyn Expr>>,
}

impl VarDecl {
    pub fn parse(parser: &mut Parser) -> Self {
        let name = parser.expect_ident();
        logln(
            LogLevel::Info,
            &format!("parse_statement variable declaration name={}", name),
        );
        let initializer = if let Token::Assign(t) = &parser.tokens()[parser.index()] {
            assert_eq!(*t, Option::<BinaryOp>::None);
            parser.bump();
            Some(parser.parse_expression())
        } else {
            None
        };
        if let Token::Semicolon = parser.tokens()[parser.index()] {
            parser.bump();
        }
        Self { name, initializer }
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
