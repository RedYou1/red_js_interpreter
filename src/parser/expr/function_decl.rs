use std::rc::Rc;

use crate::{
    Code, CodeResult, Environment, LogLevel, Prototype, new_generator, new_runnable,
    parser::{expr::Expr, lexer::Token, parser::Parser},
};

#[derive(Debug)]
pub struct FunctionDecl {
    pub name: &'static str,
    pub params: Vec<String>,
    pub excess: Option<String>,
    pub parameter_initializers: Vec<Box<dyn Expr>>,
    pub body: Rc<[Box<dyn Expr>]>,
    pub generator: bool,
    pub insert: bool,
    pub lexical_this: bool,
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
        parser.env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!("Entering FunctionDecl::parse name={}", name)
        });
        // Expect LParen
        parser.skip_to(Token::LParen);
        if !matches!(parser.tokens()[parser.index()], Token::LParen) {
            parser.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "FunctionDecl::parse expected '(' at index {} but found {:?}",
                    parser.index(),
                    parser.tokens()[parser.index()]
                )
            });
            panic!("expected '('");
        }
        parser.bump();
        let mut parameter_initializers = Vec::new();
        let (params, excess) = parser.parse_function_parameters(&mut parameter_initializers);
        parser.skip_to(Token::LBrace);
        let body = Rc::from(parser.parse_block());
        Self {
            name,
            params,
            excess,
            parameter_initializers,
            body,
            generator,
            insert,
            lexical_this: false,
        }
    }
}

impl Expr for FunctionDecl {
    fn compile(&self, env: Environment) -> Vec<Code> {
        let function_proto = Prototype::find(env.mem.clone(), &stringify!(Function).into())
            .1
            .borrow()
            .unwrap_proto("expr::FunctionDecl for Function");

        let generator = self.generator;
        let params = self.params.clone();
        let excess = self.excess.clone();
        let parameter_initializers = self
            .parameter_initializers
            .iter()
            .map(|initializer| initializer.duplicate())
            .collect::<Vec<_>>();
        let body = self.body.clone();
        let insert = self.insert;
        let lexical_this = self.lexical_this;

        env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!(
                "Entering FunctionDecl::compile name={} generator={}",
                self.name, self.generator
            )
        });
        let name = self.name;
        vec![Box::new(move |env, _| {
            env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!(
                    "Entering FunctionDecl::execute name={} generator={}",
                    name, generator
                )
            });

            let my_mem = Prototype::new_child(env.mem.clone(), None, []);
            let lexical_this = if lexical_this {
                Some(Prototype::find(env.mem.clone(), &"this".into()).1)
            } else {
                None
            };
            let function_environment = Environment {
                mem: my_mem.clone(),
                logger: env.logger.clone(),
            };
            let mut code: Vec<Code> = parameter_initializers
                .iter()
                .flat_map(|initializer| initializer.compile(function_environment.clone()))
                .collect();
            code.extend(
                body.iter()
                    .flat_map(|stmt| stmt.compile(function_environment.clone())),
            );

            let js_func = if generator {
                new_generator(
                    function_proto.clone(),
                    name,
                    crate::Generator {
                        params: params.clone(),
                        excess: excess.clone(),
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
                        excess: excess.clone(),
                        code,
                        mem: my_mem.clone(),
                        lexical_this: lexical_this.clone(),
                    },
                )
            };

            if insert {
                env.mem
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
            excess: self.excess.clone(),
            parameter_initializers: self
                .parameter_initializers
                .iter()
                .map(|initializer| initializer.duplicate())
                .collect(),
            body: self.body.iter().map(|t| t.duplicate()).collect(),
            generator: self.generator,
            insert: self.insert,
            lexical_this: self.lexical_this,
        })
    }
}
