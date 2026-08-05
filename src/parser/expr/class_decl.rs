use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        expr::{self, Expr, FunctionDecl},
        lexer::Token,
        parser::Parser,
    },
};

#[derive(Debug)]
pub struct ClassDecl {
    pub name: &'static str,
    pub super_class: Option<Box<dyn Expr>>,
    pub methods: Rc<[FunctionDecl]>,
}

impl ClassDecl {
    pub fn parse(parser: &mut Parser) -> Self {
        let name = if let Token::Ident(_) = parser.tokens()[parser.index()] {
            parser.expect_ident().leak()
        } else {
            panic!("expected class name");
        };
        logln(
            LogLevel::Info,
            &format!("parse_statement class name={}", name),
        );
        let super_class = if let Token::Ident(ident) = &parser.tokens()[parser.index()] {
            if ident.eq("extends") {
                parser.bump();
                Some(parser.parse_call_or_primary(true))
            } else {
                None
            }
        } else {
            None
        };
        if !matches!(parser.tokens()[parser.index()], Token::LBrace) {
            panic!("expected '{{' after class name");
        }
        parser.bump();
        let mut methods = Vec::new();
        while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
            if let Token::Ident(_) = parser.tokens()[parser.index()] {
                methods.push(expr::FunctionDecl::parse(parser, false));
                continue;
            }
            parser.bump();
        }
        let class = Self {
            name,
            super_class,
            methods: Rc::from(methods),
        };
        if let Token::RBrace = parser.tokens()[parser.index()] {
            parser.bump();
        }
        class
    }
}

impl Expr for ClassDecl {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let super_constructor: Code = if let Some(ref expr) = self.super_class {
            expr.compile_expr(mem.clone())
        } else {
            Box::new(|_, _| CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined))))
        };

        let constructor_method = self.methods.iter().find(|m| m.name == "constructor");

        let constructor_runnable = if let Some(constructor) = constructor_method {
            constructor.compile_expr(mem.clone())
        } else {
            FunctionDecl {
                name: "constructor",
                params: vec![],
                body: Rc::new([]),
                generator: false,
                insert: false,
            }
            .compile_expr(mem.clone())
        };

        let name = self.name;
        let methodes: Vec<(&'static str, Code)> = self
            .methods
            .iter()
            .map(|methode| (methode.name, methode.compile_expr(mem.clone())))
            .collect();
        let mem = mem.clone();
        Box::new(move |proto, i| {
            logln(
                LogLevel::Trace,
                &format!("Expr::ClassDecl executing name={}", name),
            );
            let outer_proto = proto.clone();

            let class_proto = Prototype::new_child(
                if let JsValue::Prototype(super_func_proto) =
                    inline_borrow!(handle_return!(super_constructor(proto.clone(), i)))
                    && let JsValue::Prototype(super_proto_obj) = inline_borrow!(
                        Prototype::find(super_func_proto.clone(), &"prototype".into()).1
                    )
                {
                    super_proto_obj
                } else {
                    Prototype::find(proto.clone(), &stringify!(Object).into())
                        .1
                        .borrow()
                        .unwrap_proto("expr::ClassDecl for Object")
                },
                Some(name),
                [],
            );
            class_proto.borrow_mut().properties.insert(
                "constructor".into(),
                handle_return!(constructor_runnable(
                    outer_proto.clone(),
                    &mut CodeIndex::new()
                )),
            );
            class_proto.borrow_mut().properties.insert(
                "prototype".into(),
                Rc::new(RefCell::new(JsValue::Prototype(class_proto.clone()))),
            );

            for (methode_name, methode_code) in methodes.iter() {
                class_proto.borrow_mut().properties.insert(
                    (*methode_name).into(),
                    handle_return!(methode_code(outer_proto.clone(), &mut CodeIndex::new())),
                );
            }

            logln(
                LogLevel::Trace,
                &format!("Exiting Expr::ClassDecl {class_proto:?}"),
            );
            let res = Rc::new(RefCell::new(JsValue::Prototype(class_proto)));
            mem.borrow_mut().properties.insert(name.into(), res.clone());
            CodeResult::Normal(res)
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name,
            super_class: self
                .super_class
                .as_ref()
                .map(|t| t.as_ref().duplicate_expr()),
            methods: self.methods.clone(),
        })
    }
}
