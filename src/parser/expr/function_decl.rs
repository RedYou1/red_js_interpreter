use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, LogLevel, Prototype, logln, new_generator, new_runnable,
    parser::{
        expr::Expr,
        lexer::Token,
        parser::{ParseError, Parser},
        stmt::Stmt,
    },
};

pub struct FunctionDecl {
    pub name: &'static str,
    pub params: Vec<String>,
    pub body: Rc<[Box<dyn Stmt>]>,
    pub generator: bool,
    pub insert: bool,
}

impl FunctionDecl {
    pub fn parse(parser: &mut Parser, insert: bool) -> Result<Self, ParseError> {
        let generator = if let Token::Star = parser.current() {
            parser.bump();
            true
        } else {
            false
        };
        let name = if let Token::Ident(s) = &parser.current() {
            let name = s.clone().leak();
            parser.bump();
            name
        } else {
            "anonymous function"
        };
        logln(
            LogLevel::Info,
            &format!("parse_statement function name={}", name),
        );
        // Expect LParen
        parser.skip_to(Token::LParen);
        parser.expect_and_bump(Token::LParen, "expected '('")?;
        let params = parser.parse_param_list()?;
        parser.skip_to(Token::LBrace);
        let body = Rc::from(parser.parse_block_body()?);
        Ok(Self {
            name,
            params,
            body,
            generator,
            insert,
        })
    }
}

impl Expr for FunctionDecl {
    fn compile_expr(&self, mem: Rc<RefCell<Prototype>>) -> Code {
        let function_proto = Prototype::find(mem.clone(), &stringify!(Function).into())
            .1
            .borrow()
            .unwrap_proto("expr::FunctionDecl for Function");

        let generator = self.generator;
        let params = self.params.clone();
        let body = self.body.clone();
        let insert = self.insert;

        logln(
            LogLevel::Info,
            &format!(
                "FunctionDecl::compile_expr name={} generator={}",
                self.name, self.generator
            ),
        );
        let name = self.name;
        Box::new(move |proto, _| {
            logln(
                LogLevel::Trace,
                &format!(
                    "Entering FunctionDecl execution name={} generator={}",
                    name, generator
                ),
            );

            let my_mem = Prototype::new_child(proto.clone(), None, []);
            let code: Vec<Code> = body
                .iter()
                .flat_map(|stmt| stmt.compile_stmt(my_mem.clone()))
                .collect();

            let js_func = if generator {
                new_generator(
                    function_proto.clone(),
                    name,
                    crate::Generator {
                        params: params.clone(),
                        excess: None,
                        code: Rc::from(code),
                        mem: my_mem.clone(),
                    },
                )
            } else {
                new_runnable(
                    function_proto.clone(),
                    name,
                    crate::Runnable {
                        params: params.clone(),
                        excess: None,
                        code,
                        mem: my_mem.clone(),
                    },
                )
            };

            if insert {
                proto
                    .borrow_mut()
                    .properties
                    .insert(name.into(), js_func.clone());
            }
            CodeResult::Normal(js_func.clone())
        })
    }
    fn duplicate_expr(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name,
            params: self.params.clone(),
            body: self.body.iter().map(|t| t.duplicate_stmt()).collect(),
            generator: self.generator,
            insert: self.insert,
        })
    }
}
