use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeResult, LogLevel, Prototype, logln, new_generator, new_runnable,
    parser::{expr::Expr, lexer::Token, parser::Parser},
};

#[derive(Debug)]
pub struct FunctionDecl {
    pub name: &'static str,
    pub params: Vec<String>,
    pub body: Rc<[Box<dyn Expr>]>,
    pub generator: bool,
    pub insert: bool,
}

impl FunctionDecl {
    pub fn parse(parser: &mut Parser, insert: bool) -> Self {
        let generator = if let Token::Star = parser.tokens()[parser.index()] {
            parser.bump();
            true
        } else {
            false
        };
        let name = if let Token::Ident(s) = &parser.tokens()[parser.index()] {
            let name = s.clone().leak();
            parser.bump();
            name
        } else {
            "anonymous function"
        };
        logln(
            LogLevel::Info,
            &format!("parse_FuncDecl function name={}", name),
        );
        // Expect LParen
        parser.skip_to(Token::LParen);
        if !matches!(parser.tokens()[parser.index()], Token::LParen) {
            panic!("expected '('");
        }
        parser.bump();
        let params = parser.parse_param_list();
        parser.skip_to(Token::LBrace);
        let body = Rc::from(parser.parse_block());
        Self {
            name,
            params,
            body,
            generator,
            insert,
        }
    }
}

impl Expr for FunctionDecl {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
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
        vec![Box::new(move |proto, _| {
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
                .flat_map(|stmt| stmt.compile(my_mem.clone()))
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
        })]
    }
    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            name: self.name,
            params: self.params.clone(),
            body: self.body.iter().map(|t| t.duplicate()).collect(),
            generator: self.generator,
            insert: self.insert,
        })
    }
}
