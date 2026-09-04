use std::io::{Write, stdout};
use std::rc::Rc;

use crate::parser::ast::*;
use crate::parser::expr::{self, Expr};
use crate::parser::lexer::{Lexer, Token};
use crate::{Environment, JsValue, LogLevel};

pub struct Parser {
    pub env: Environment,
    tokens: Vec<Token>,
    index: usize,
    can_multi: bool,
}

impl Parser {
    pub fn new(input: &str, env: Environment) -> Self {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::with_capacity(input.len());
        loop {
            let value = lexer.next_token(tokens.as_slice());
            if value == Token::Eof {
                tokens.push(value);
                break;
            }
            tokens.push(value);
        }
        let parser = Self {
            tokens,
            index: 0,
            can_multi: true,
            env,
        };
        parser.env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
            format!(
                "Parser::new input_len={} token_len={} first_5={:?}",
                input.len(),
                parser.tokens.len(),
                parser.tokens[parser.index..]
                    .iter()
                    .take(5)
                    .collect::<Vec<_>>(),
            )
        });
        parser
    }

    pub fn bump(&mut self) {
        let prev = &self.tokens[self.index];
        assert_ne!(*prev, Token::Eof);
        self.index += 1;
        self.env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
            format!("Parser::bump {:?} -> {:?}", prev, self.tokens[self.index])
        });
    }

    pub const fn index(&self) -> usize {
        self.index
    }
    pub const fn tokens(&self) -> &[Token] {
        self.tokens.as_slice()
    }

    pub fn expect_ident(&mut self) -> String {
        match &self.tokens[self.index] {
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                name
            }
            t => {
                self.env.logger.borrow_mut().logln(LogLevel::Error, &|| {
                    format!(
                        "Parser::expect_ident expected identifier at index {} but found {:?}",
                        self.index, t
                    )
                });
                panic!("expected identifier, got {:?}", t);
            }
        }
    }

    pub fn skip_to(&mut self, token: Token) {
        while self.tokens[self.index] != token && self.tokens[self.index] != Token::Eof {
            self.bump();
        }
    }

    pub fn parse_param_list(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        while self.tokens[self.index] != Token::RParen
            && self.tokens[self.index] != Token::Arrow
            && self.tokens[self.index] != Token::Eof
        {
            if let Token::Ident(s) = &self.tokens[self.index] {
                params.push(s.clone());
            }
            self.bump();
            if self.tokens[self.index] == Token::Comma {
                self.bump();
            }
        }
        if self.tokens[self.index] == Token::Arrow {
            return params;
        }
        if self.tokens[self.index] != Token::RParen {
            self.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                format!(
                    "Parser::parse_param_list expected ')' at index {} but found {:?}",
                    self.index, self.tokens[self.index]
                )
            });
            panic!("expected ')'");
        }
        self.bump();
        params
    }

    pub fn parse_block(&mut self) -> Vec<Box<dyn Expr>> {
        if self.tokens[self.index] != Token::LBrace {
            let res = self.parse_expression(true);

            if let Token::Semicolon = self.tokens[self.index] {
                self.bump();
            }

            return res;
        }
        assert_eq!(self.tokens[self.index], Token::LBrace);
        self.bump();
        let mut res: Vec<Box<dyn Expr>> = Vec::new();
        while !matches!(self.tokens[self.index], Token::Eof | Token::RBrace) {
            let before = self.index;
            let mut exprs = self.parse_expression(true);
            if exprs.is_empty() && self.index == before {
                if self.tokens[self.index] == Token::Comma {
                    self.bump();
                    continue;
                }
                self.env.logger.borrow_mut().logln(LogLevel::Error, &|| {
                    format!(
                        "Parser::parse_block made no progress at index {} token {:?}",
                        self.index, self.tokens[self.index]
                    )
                });
                panic!(
                    "parser made no progress inside block at token {:?}",
                    self.tokens[self.index]
                );
            }
            res.append(&mut exprs);
        }
        if let Token::RBrace = self.tokens[self.index] {
            self.bump();
        }
        res
    }

    pub fn parse_program(&mut self) -> Program {
        self.env
            .logger
            .borrow_mut()
            .logln_str(LogLevel::Info, "Entering Parser::parse_program");
        let mut body = Vec::new();
        while self.tokens[self.index] != Token::Eof {
            let current = self.tokens[self.index].clone();
            let mut stmt = self.parse_all_expressions();
            if !stmt.is_empty() {
                body.append(&mut stmt);
            } else if self.tokens[self.index] == current {
                // only advance when no statement was parsed and the token stream did not move
                self.bump();
            }
        }
        self.env.logger.borrow_mut().logln(LogLevel::Info, &|| {
            format!("Exiting Parser::parse_program body_size={}", body.len())
        });
        Program { body }
    }

    pub fn parse_all_expressions(&mut self) -> Vec<Box<dyn Expr>> {
        let mut res: Vec<Box<dyn Expr>> = Vec::new();
        while !matches!(self.tokens[self.index], Token::Eof) {
            if !self.can_multi {
                self.env.logger.borrow_mut().logln(
                    LogLevel::Fatal,
                    &||format!(
                        "Parser::parse_all_expressions entered with can_multi=false at index {} token {:?}",
                        self.index, self.tokens[self.index]
                    ),
                );
                panic!("parse_all_expressions requires can_multi");
            }
            let mut exprs = self.parse_expression(true);
            if exprs.is_empty() {
                break;
            }
            res.append(&mut exprs);
            if let Err(error) = stdout().flush() {
                self.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                    format!("Parser::parse_all_expressions failed to flush stdout: {error}")
                });
                panic!("failed to flush stdout: {error}");
            }
        }
        res
    }

    pub fn parse_expression(&mut self, value: bool) -> Vec<Box<dyn Expr>> {
        let temp = self.can_multi;
        self.can_multi = value;
        let res = self.parse_expression_inner();
        self.can_multi = temp;
        res
    }

    fn parse_expression_inner(&mut self) -> Vec<Box<dyn Expr>> {
        let mut res: Vec<Box<dyn Expr>> = Vec::new();
        loop {
            self.env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                format!(
                    "parse_expression can_multi={} current_token={:?} expressions={:?}",
                    self.can_multi, self.tokens[self.index], res
                )
            });
            match &self.tokens[self.index] {
                Token::Try => {
                    self.bump();
                    res.push(Box::new(expr::Try::parse(self)));
                }
                Token::Function => {
                    self.bump();
                    let expr = Box::new(expr::FunctionDecl::parse(self, true));
                    res.push(self.parse_postfix(expr, true));
                }
                Token::Class => {
                    self.bump();
                    res.push(Box::new(expr::ClassDecl::parse(self)));
                }
                Token::Let | Token::Const | Token::Var => {
                    self.bump();
                    res.push(Box::new(expr::VarDecl::parse(self)));
                }
                Token::If => {
                    self.bump();
                    res.push(Box::new(expr::IfExpr::parse(self)));
                }
                Token::While => res.push(Box::new(expr::LoopExpr::parse(self))),
                Token::For => res.push(Box::new(expr::LoopExpr::parse(self))),
                Token::Do => {
                    self.bump();
                    let body = self.parse_block();

                    if self.tokens[self.index] != Token::While {
                        self.env.logger.borrow_mut().logln(LogLevel::Error, &|| {
                            format!(
                                "Parser::parse_expression expected while after do body at index {}",
                                self.index
                            )
                        });
                        panic!("expected 'while' after do body");
                    }
                    self.bump();
                    if self.tokens[self.index] != Token::LParen {
                        self.env.logger.borrow_mut().logln(
                            LogLevel::Fatal,
                            &||format!(
                                "Parser::parse_expression expected '(' after while at index {} but found {:?}",
                                self.index, self.tokens[self.index]
                            ),
                        );
                        panic!("expected '(' after while");
                    }
                    self.bump();
                    let condition: Option<Box<dyn Expr>> =
                        Some(Box::new(self.parse_expression(true)));
                    if self.tokens[self.index] != Token::RParen {
                        self.env.logger.borrow_mut().logln(LogLevel::Error, &|| {
                            format!(
                                "Parser::parse_expression expected ')' after do-while at index {}",
                                self.index
                            )
                        });
                        panic!("expected ')' after while condition");
                    }
                    self.bump();
                    if self.tokens[self.index] == Token::Semicolon {
                        self.bump();
                    }

                    res.push(Box::new(expr::LoopExpr {
                        init: None,
                        body,
                        condition,
                        update: None,
                        do_first: true,
                    }));
                }
                Token::Break | Token::Continue | Token::Return | Token::Yield | Token::Throw => {
                    res.push(Box::new(expr::Return::parse(self)));
                }
                Token::Semicolon => {
                    self.bump();
                    break;
                }
                Token::Comma => {
                    if self.can_multi {
                        self.bump();
                    } else {
                        break;
                    }
                }
                Token::RBrace | Token::RParen | Token::RBracket => break,
                Token::Eof => break,
                Token::Colon => break,
                Token::Else => break,
                Token::Case | Token::Default => break,
                Token::Switch => res.push(Box::new(expr::SwitchExpr::parse(self))),
                Token::Ident(name) if let Token::Colon = self.tokens[self.index + 1] => {
                    let name = name.clone();
                    self.bump();
                    self.bump();
                    res.push(Box::new(expr::Label {
                        name,
                        code: self.parse_block(),
                    }));
                }
                _ if (matches!(self.tokens[self.index], Token::LParen)
                    && self.tokens[self.index..].contains(&Token::Arrow)
                    && self.tokens[self.index..]
                        .splitn(2, |t| *t == Token::Arrow)
                        .next()
                        .is_some_and(|params| {
                            params.iter().all(|t| {
                                matches!(
                                    *t,
                                    Token::LParen | Token::RParen | Token::Ident(_) | Token::Comma
                                )
                            })
                        }))
                    || (matches!(&self.tokens[self.index], Token::Ident(name) if name == "async")
                        && matches!(self.tokens.get(self.index + 1), Some(Token::LParen))
                        && self.tokens[self.index + 1..]
                            .splitn(2, |t| *t == Token::Arrow)
                            .next()
                            .is_some_and(|params| {
                                params.iter().all(|t| {
                                    matches!(
                                        *t,
                                        Token::LParen
                                            | Token::RParen
                                            | Token::Ident(_)
                                            | Token::Comma
                                    )
                                })
                            }))
                    || (matches!(self.tokens[self.index], Token::Ident(_))
                        && self.tokens[self.index + 1] == Token::Arrow) =>
                {
                    if matches!(&self.tokens[self.index], Token::Ident(name) if name == "async") {
                        self.bump();
                    }
                    let params = self.parse_param_list();
                    assert_eq!(self.tokens[self.index], Token::Arrow);
                    self.bump();
                    let body: Rc<[Box<dyn Expr>]> = if self.tokens[self.index] == Token::LBrace {
                        Rc::from(self.parse_block())
                    } else {
                        Rc::new([Box::new(expr::Return {
                            expr: Some(Box::new(self.parse_expression(true))),
                            rtype: expr::ReturnType::Return,
                        })])
                    };
                    res.push(Box::new(expr::FunctionDecl {
                        name: "anonymous function",
                        params,
                        body,
                        generator: false,
                        insert: false,
                    }))
                }
                _ => {
                    let expr = expr::Operator::parse(self);
                    match &self.tokens[self.index] {
                        Token::Assign(t) => {
                            let t = t.clone();
                            self.bump();
                            let rhs: Box<dyn Expr> = if let Some(op) = t {
                                Box::new(expr::Operator {
                                    left: expr.duplicate(),
                                    op,
                                    right: Box::new(self.parse_expression(false)),
                                })
                            } else {
                                Box::new(self.parse_expression(false))
                            };
                            res.push(Box::new(expr::Assign {
                                target: expr,
                                value: rhs,
                            }))
                        }
                        _ => res.push(expr),
                    }
                }
            }
            if !self.can_multi {
                break;
            }
        }
        res
    }

    pub fn parse_primary(&mut self) -> Box<dyn Expr> {
        self.env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
            format!(
                "Entering Parser::parse_primary cur={:?}",
                self.tokens[self.index]
            )
        });
        match &self.tokens[self.index] {
            Token::Typeof => {
                self.bump();
                Box::new(expr::Typeof::parse(self))
            }
            Token::Void => {
                self.bump();
                let mut expr = self.parse_expression(true);
                expr.push(Box::new(expr::ConstObj {
                    obj: JsValue::Undefined,
                }));
                Box::new(expr)
            }
            Token::Delete => {
                self.bump();
                Box::new(expr::Delete {
                    expr: self.parse_call_or_primary(false),
                })
            }
            Token::LBrace => Box::new(expr::Object::parse(self)),
            Token::LBracket | Token::LParen => {
                let end = if let Token::LBracket = self.tokens[self.index] {
                    Token::RBracket
                } else {
                    Token::RParen
                };
                self.bump();
                let mut elements: Vec<Box<dyn Expr>> = Vec::new();
                while self.tokens[self.index] != Token::Eof && self.tokens[self.index] != end {
                    let before = self.index;
                    let mut exprs = self.parse_expression(true);
                    if exprs.is_empty() && self.index == before {
                        self.env.logger.borrow_mut().logln(
                            LogLevel::Fatal,
                            &||format!(
                                "Parser::parse_primary grouped expression made no progress at index {} token {:?}",
                                self.index, self.tokens[self.index]
                            ),
                        );
                        panic!(
                            "parser made no progress in grouped expression at token {:?}",
                            self.tokens[self.index]
                        );
                    }
                    elements.append(&mut exprs);
                }
                if self.tokens[self.index] != end {
                    self.env.logger.borrow_mut().logln(LogLevel::Fatal, &|| {
                        format!(
                            "Parser::parse_primary expected {:?} at index {} but found {:?}",
                            end, self.index, self.tokens[self.index]
                        )
                    });
                    panic!("expected ']' at end of array literal");
                }
                self.bump();
                if end == Token::RBracket {
                    Box::new(expr::Array { elems: elements })
                } else if elements.len() == 1 {
                    elements.pop().unwrap()
                } else {
                    Box::new(elements)
                }
            }
            Token::New => {
                self.bump();
                Box::new(expr::New::parse(self))
            }
            Token::Function => {
                // simple function expression: function name? (params) { ... }
                self.bump();
                Box::new(expr::FunctionDecl::parse(self, false))
            }
            Token::Class => {
                self.bump();
                Box::new(expr::ClassDecl::parse(self))
            }
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                Box::new(expr::Identifier { name })
            }
            Token::BigInt(n) => {
                let value = *n;
                self.bump();
                Box::new(expr::ConstBigInt { num: value })
            }
            Token::Number(n) => {
                let value = *n;
                self.bump();
                Box::new(expr::ConstNumber { num: value })
            }
            Token::TemplateStart => {
                self.bump();
                Box::new(expr::TemplateLiteral::parse(self))
            }
            Token::Str(s) => {
                let value = s.clone();
                self.bump();
                Box::new(expr::ConstString { s: value })
            }
            Token::True => {
                self.bump();
                Box::new(expr::ConstBoolean { b: true })
            }
            Token::False => {
                self.bump();
                Box::new(expr::ConstBoolean { b: false })
            }
            Token::Undefined => {
                self.bump();
                Box::new(expr::ConstObj {
                    obj: JsValue::Undefined,
                })
            }
            Token::Null => {
                self.bump();
                Box::new(expr::ConstObj { obj: JsValue::Null })
            }
            Token::Minus if let Token::BigInt(n) = &self.tokens[self.index + 1] => {
                let value = *n;
                self.bump();
                self.bump();
                Box::new(expr::ConstBigInt { num: -value })
            }
            Token::Minus if let Token::Number(n) = &self.tokens[self.index + 1] => {
                let value = *n;
                self.bump();
                self.bump();
                Box::new(expr::ConstNumber { num: -value })
            }
            Token::Regex(source, flags) => {
                let source = source.clone();
                let flags = flags.clone();
                self.bump();
                Box::new(expr::ConstRegex { source, flags })
            }
            e => {
                let name = e.as_keyword().to_owned();
                self.bump();
                Box::new(expr::Identifier { name })
            }
        }
    }

    pub fn parse_call_or_primary(&mut self, root: bool) -> Box<dyn Expr> {
        self.env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
            format!(
                "Entering Parser::parse_call_or_primary cur={:?} root={:?}",
                self.tokens[self.index], root
            )
        });
        let expr = self.parse_primary();
        self.parse_postfix(expr, root)
    }

    fn parse_postfix(&mut self, mut expr: Box<dyn Expr>, root: bool) -> Box<dyn Expr> {
        loop {
            match self.tokens[self.index] {
                Token::QMark if root => {
                    expr = Box::new(expr::ConditionalOp::parse(self, expr));
                }
                Token::Dot => {
                    self.bump();
                    let prop = self.tokens[self.index].as_keyword().to_owned();
                    self.bump();
                    expr = Box::new(expr::Member {
                        object: expr,
                        property: Box::new(expr::ConstString { s: prop }),
                    });
                }
                Token::LBracket => {
                    self.bump();
                    let index = Box::new(self.parse_expression(true));
                    if self.tokens[self.index] != Token::RBracket {
                        self.env.logger.borrow_mut().logln(
                            LogLevel::Fatal,
                            &||format!(
                                "Parser::parse_call_or_primary expected ']' at index {} but found {:?}",
                                self.index, self.tokens[self.index]
                            ),
                        );
                        panic!("expected ']'");
                    }
                    self.bump();
                    expr = Box::new(expr::Member {
                        object: expr,
                        property: index,
                    });
                }
                Token::LParen => {
                    self.bump();
                    expr = Box::new(expr::Call {
                        func: expr,
                        args: self.parse_expression(true),
                    });
                    if self.tokens[self.index] == Token::RParen {
                        self.bump();
                    }
                }
                Token::PlusPlus => {
                    self.bump();
                    expr = Box::new(expr::Postfix { expr, inc: true });
                }
                Token::MinusMinus => {
                    self.bump();
                    expr = Box::new(expr::Postfix { expr, inc: false });
                }
                _ => break,
            }
        }
        expr
    }
}

pub fn parse(input: &str, env: Environment) -> Program {
    let mut p = Parser::new(input, env);
    p.parse_program()
}
