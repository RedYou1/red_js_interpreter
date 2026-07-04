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
    pub body: Vec<Box<dyn Stmt>>,
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
        let body = parser.parse_block_body()?;
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

        let my_mem = Prototype::new_child(
            mem.clone(),
            Some(format!("root memory of {}", self.name).leak()),
            [],
        );
        let code: Vec<Code> = self
            .body
            .iter()
            .flat_map(|stmt| stmt.compile_stmt(my_mem.clone()))
            .collect();

        let generator = self.generator;

        logln(
            LogLevel::Info,
            &format!(
                "FunctionDecl::compile_expr name={} generator={}",
                self.name, self.generator
            ),
        );
        let js_func = if self.generator {
            new_generator(
                function_proto,
                self.name,
                crate::Generator {
                    params: self.params.clone(),
                    excess: None,
                    code: Rc::from(code),
                    mem: my_mem,
                },
            )
        } else {
            new_runnable(
                function_proto,
                self.name,
                crate::Runnable {
                    params: self.params.clone(),
                    excess: None,
                    code,
                    mem: my_mem,
                },
            )
        };

        if self.insert {
            mem.borrow_mut()
                .properties
                .insert(self.name.into(), js_func.clone());
        }
        let name = self.name;
        Box::new(move |_, _| {
            logln(
                LogLevel::Trace,
                &format!(
                    "Entering FunctionDecl execution name={} generator={}",
                    name, generator
                ),
            );
            CodeResult::Normal(js_func.clone())
        })
    }
}
