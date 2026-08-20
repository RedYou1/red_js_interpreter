use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Write, stdout};
use std::rc::Rc;

use crate::parser::ast::*;
use crate::parser::expr::{self, Expr};
use crate::parser::lexer::{Lexer, Token};
use crate::{JsValue, LogLevel, Prototype, logln};

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
    can_multi: bool,
}

impl Parser {
    pub fn new(input: &str) -> Self {
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
        };
        logln(
            LogLevel::Trace,
            &format!(
                "Parser::new input_len={} token_len={} first_5={:?}",
                input.len(),
                parser.tokens.len(),
                parser.tokens[parser.index..]
                    .iter()
                    .take(5)
                    .collect::<Vec<_>>(),
            ),
        );
        parser
    }

    pub fn bump(&mut self) {
        let prev = &self.tokens[self.index];
        assert_ne!(*prev, Token::Eof);
        self.index += 1;
        logln(
            LogLevel::Trace,
            &format!("Parser::bump {:?} -> {:?}", prev, self.tokens[self.index]),
        );
    }

    pub const fn index(&self) -> usize {
        self.index
    }
    pub const fn tokens(&self) -> &[Token] {
        self.tokens.as_slice()
    }
    pub const fn can_multi(&self) -> bool {
        self.can_multi
    }
    pub fn set_can_multi<R>(&mut self, value: bool, action: impl FnOnce(&mut Parser) -> R) -> R {
        let temp = self.can_multi;
        self.can_multi = value;
        let res = action(self);
        self.can_multi = temp;
        res
    }

    pub fn expect_ident(&mut self) -> String {
        match &self.tokens[self.index] {
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                name
            }
            t => {
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
            panic!("expected ')'");
        }
        self.bump();
        params
    }

    pub fn parse_block(&mut self) -> Vec<Box<dyn Expr>> {
        if self.tokens[self.index] != Token::LBrace {
            return self.parse_expression();
        }
        assert_eq!(self.tokens[self.index], Token::LBrace);
        self.bump();
        let mut res: Vec<Box<dyn Expr>> = Vec::new();
        while !matches!(self.tokens[self.index], Token::Eof | Token::RBrace) {
            let before = self.index;
            let mut exprs = self.parse_expression();
            if exprs.is_empty() && self.index == before {
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
        logln(LogLevel::Info, "parse_program start");
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
        logln(
            LogLevel::Info,
            &format!("parse_program finished body_size={}", body.len()),
        );
        Program { body }
    }

    pub fn parse_all_expressions(&mut self) -> Vec<Box<dyn Expr>> {
        let mut res: Vec<Box<dyn Expr>> = Vec::new();
        while !matches!(self.tokens[self.index], Token::Eof) {
            assert!(self.can_multi);
            let mut exprs = self.parse_expression();
            if exprs.is_empty() {
                break;
            }
            res.append(&mut exprs);
            stdout().flush().unwrap();
        }
        res
    }

    pub fn parse_expression(&mut self) -> Vec<Box<dyn Expr>> {
        let mut res: Vec<Box<dyn Expr>> = Vec::new();
        loop {
            logln(
                LogLevel::Trace,
                &format!(
                    "parse_expression cant_multi={:?} current_token={:?} res={:?}",
                    self.can_multi, self.tokens[self.index], res
                ),
            );
            match &self.tokens[self.index] {
                Token::Try => {
                    self.bump();
                    res.push(Box::new(expr::Try::parse(self)));
                }
                Token::Function => {
                    self.bump();
                    res.push(Box::new(expr::FunctionDecl::parse(self, true)));
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
                    if self.tokens[self.index] != Token::LBrace {
                        panic!("expected '{{' for do body");
                    }
                    let body = self.parse_block();

                    if self.tokens[self.index] != Token::While {
                        panic!("expected 'while' after do body");
                    }
                    self.bump();
                    if self.tokens[self.index] != Token::LParen {
                        panic!("expected '(' after while");
                    }
                    self.bump();
                    let condition: Option<Box<dyn Expr>> = Some(Box::new(self.parse_expression()));
                    if self.tokens[self.index] != Token::RParen {
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
                        assert!(!res.is_empty());
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
                _ if let [params, _] = &self.tokens[self.index..]
                    .splitn(2, |t| *t == Token::Arrow)
                    .collect::<Vec<_>>()[..]
                    && params.iter().all(|t| {
                        matches!(
                            *t,
                            Token::LParen | Token::RParen | Token::Ident(_) | Token::Comma
                        )
                    }) =>
                {
                    let params = self.parse_param_list();
                    assert_eq!(self.tokens[self.index], Token::Arrow);
                    self.bump();
                    let body: Rc<[Box<dyn Expr>]> = if self.tokens[self.index] == Token::LBrace {
                        Rc::from(self.parse_block())
                    } else {
                        Rc::new([Box::new(expr::Return {
                            expr: Some(Box::new(self.parse_expression())),
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
                        Token::InstanceOf => {
                            self.bump();
                            let class = self.parse_primary();
                            res.push(Box::new(expr::Call {
                                args: vec![expr],
                                func: Box::new(expr::Member {
                                    object: class,
                                    property: Box::new(expr::Member {
                                        object: Box::new(expr::Identifier {
                                            name: stringify!(Symbol).to_owned(),
                                        }),
                                        property: Box::new(expr::ConstString {
                                            s: "hasInstance".to_owned(),
                                        }),
                                    }),
                                }),
                            }))
                        }
                        Token::Assign(t) => {
                            let t = t.clone();
                            self.bump();
                            let rhs: Box<dyn Expr> = if let Some(op) = t {
                                Box::new(expr::Operator {
                                    left: expr.duplicate(),
                                    op,
                                    right: self.set_can_multi(false, |parser| {
                                        Box::new(parser.parse_expression())
                                    }),
                                })
                            } else {
                                self.set_can_multi(false, |parser| {
                                    Box::new(parser.parse_expression())
                                })
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
        logln(
            LogLevel::Trace,
            &format!("parse_primary cur={:?}", self.tokens[self.index]),
        );
        match &self.tokens[self.index] {
            Token::Typeof => {
                self.bump();
                Box::new(expr::Typeof::parse(self))
            }
            Token::Void => {
                self.bump();
                let mut expr = self.parse_expression();
                expr.push(Box::new(expr::ConstObj {
                    obj: JsValue::Undefined,
                }));
                Box::new(expr)
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
                    let mut exprs = self.set_can_multi(true, |parser| parser.parse_expression());
                    if exprs.is_empty() && self.index == before {
                        panic!(
                            "parser made no progress in grouped expression at token {:?}",
                            self.tokens[self.index]
                        );
                    }
                    elements.append(&mut exprs);
                }
                if self.tokens[self.index] != end {
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
            Token::Regex(s) => {
                let value = s.clone();
                self.bump();
                //TODO do Regex
                Box::new(expr::ConstObj {
                    obj: JsValue::Prototype(Rc::new(RefCell::new(Prototype {
                        name: None,
                        properties: HashMap::from([(
                            "value".into(),
                            Rc::new(RefCell::new(value.into())),
                        )]),
                        formating: false,
                    }))),
                })
            }
            e => {
                panic!("unexpected token in parse_primary: {:?}", e);
            }
        }
    }

    pub fn parse_call_or_primary(&mut self, root: bool) -> Box<dyn Expr> {
        logln(
            LogLevel::Trace,
            &format!(
                "parse_call_or_primary cur={:?} root={:?}",
                self.tokens[self.index], root
            ),
        );
        let mut expr = self.parse_primary();
        loop {
            match self.tokens[self.index] {
                Token::QMark if root => {
                    expr = Box::new(expr::ConditionalOp::parse(self, expr));
                }
                Token::Dot => {
                    self.bump();
                    if let Token::Ident(name) = &self.tokens[self.index] {
                        let prop = name.clone();
                        self.bump();
                        expr = Box::new(expr::Member {
                            object: expr,
                            property: Box::new(expr::ConstString { s: prop }),
                        });
                    } else {
                        panic!(
                            "expected property name after '.', got {:?}",
                            self.tokens[self.index]
                        );
                    }
                }
                Token::LBracket => {
                    self.bump();
                    let index = Box::new(self.parse_expression());
                    if self.tokens[self.index] != Token::RBracket {
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
                        args: self.set_can_multi(true, |parser| parser.parse_expression()),
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

pub fn parse(input: &str) -> Program {
    let mut p = Parser::new(input);
    p.parse_program()
}
