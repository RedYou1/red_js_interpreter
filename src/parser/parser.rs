use std::rc::Rc;

use crate::parser::expr::{self, Expr};
use crate::parser::lexer::{Lexer, Token};
use crate::parser::stmt::Stmt;
use crate::parser::{ast::*, stmt};
use crate::{LogLevel, logln};

#[derive(Debug)]
pub struct ParseError(pub String);

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    cur: Token,
    peek: Token,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let cur = lexer.next_token();
        let peek = lexer.next_token();
        let parser = Parser { lexer, cur, peek };
        logln(
            LogLevel::Trace,
            &format!(
                "Parser::new input_len={} cur={:?} peek={:?}",
                input.len(),
                parser.cur,
                parser.peek
            ),
        );
        parser
    }

    pub fn bump(&mut self) {
        let prev = self.cur.clone();
        self.cur = std::mem::replace(&mut self.peek, self.lexer.next_token());
        logln(
            LogLevel::Trace,
            &format!("Parser::bump {:?} -> {:?}", prev, self.cur),
        );
    }

    pub const fn current(&self) -> &Token {
        &self.cur
    }

    pub const fn peek(&self) -> &Token {
        &self.peek
    }

    pub fn expect_ident(&mut self) -> Result<String, ParseError> {
        match &self.cur {
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                Ok(name)
            }
            t => {
                let msg = format!("expected identifier, got {:?}", t);
                logln(LogLevel::Error, &msg);
                Err(ParseError(msg))
            }
        }
    }

    pub fn skip_to(&mut self, token: Token) {
        while self.cur != token && self.cur != Token::Eof {
            self.bump();
        }
    }

    pub fn parse_param_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut params = Vec::new();
        while self.cur != Token::RParen && self.cur != Token::Eof {
            if let Token::Ident(s) = &self.cur {
                params.push(s.clone());
            }
            self.bump();
            if self.cur == Token::Comma {
                self.bump();
            }
        }
        if self.cur != Token::RParen {
            return Err(ParseError("expected ')'".into()));
        }
        self.bump();
        Ok(params)
    }

    pub fn expect_and_bump(&mut self, token: Token, msg: &str) -> Result<(), ParseError> {
        if self.cur != token {
            return Err(ParseError(msg.into()));
        }
        self.bump();
        Ok(())
    }

    pub fn parse_block_body(&mut self) -> Result<Vec<Box<dyn Stmt>>, ParseError> {
        if self.cur != Token::LBrace {
            return Err(ParseError("expected '{'".into()));
        }
        self.bump();
        let mut body: Vec<Box<dyn Stmt>> = Vec::new();
        while self.cur != Token::RBrace && self.cur != Token::Eof {
            if let Token::Semicolon = self.cur {
                self.bump();
                continue;
            }
            let expr = self.parse_statement()?;
            if let Some(expr) = expr {
                body.push(expr);
                if self.cur == Token::Semicolon {
                    self.bump();
                }
            }
        }
        if self.cur == Token::RBrace {
            self.bump();
        }
        Ok(body)
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        logln(LogLevel::Info, "parse_program start");
        let mut body = Vec::new();
        while self.cur != Token::Eof {
            let current = self.cur.clone();
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
            } else if self.cur == current {
                // only advance when no statement was parsed and the token stream did not move
                self.bump();
            }
        }
        logln(
            LogLevel::Info,
            &format!("parse_program finished body_size={}", body.len()),
        );
        Ok(Program { body })
    }

    pub fn parse_statement(&mut self) -> Result<Option<Box<dyn Stmt>>, ParseError> {
        logln(
            LogLevel::Trace,
            &format!("parse_statement cur={:?}", self.cur),
        );
        match &self.cur {
            Token::Function => {
                self.bump();
                Ok(Some(Box::new(expr::FunctionDecl::parse(self, true)?)))
            }
            Token::Class => {
                self.bump();
                Ok(Some(Box::new(expr::ClassDecl::parse(self)?)))
            }
            Token::Let | Token::Const | Token::Var => {
                self.bump();
                Ok(Some(Box::new(expr::VarDecl::parse(self)?)))
            }
            Token::If => {
                self.bump();
                Ok(Some(Box::new(stmt::IfStmt::parse(self)?)))
            }
            Token::While => Ok(Some(Box::new(stmt::LoopStmt::parse(self)?))),
            Token::For => Ok(Some(Box::new(stmt::LoopStmt::parse(self)?))),
            Token::Do => {
                self.bump();
                if self.cur != Token::LBrace {
                    return Err(ParseError("expected '{' for do body".into()));
                }
                let body = self.parse_block_body()?;

                if self.cur != Token::While {
                    return Err(ParseError("expected 'while' after do body".into()));
                }
                self.bump();
                if self.cur != Token::LParen {
                    return Err(ParseError("expected '(' after while".into()));
                }
                self.bump();
                let condition = Some(self.parse_expression()?);
                if self.cur != Token::RParen {
                    return Err(ParseError("expected ')' after while condition".into()));
                }
                self.bump();
                if self.cur == Token::Semicolon {
                    self.bump();
                }

                Ok(Some(Box::new(stmt::LoopStmt {
                    init: None,
                    body,
                    condition,
                    update: None,
                    do_first: true,
                })))
            }
            Token::Break | Token::Continue | Token::Return | Token::Yield => {
                Ok(Some(Box::new(expr::Return::parse(self)?)))
            }
            Token::Semicolon => {
                self.bump();
                Ok(None)
            }
            Token::Eof => Ok(None),
            _ => Ok(Some(self.parse_expression()?)),
        }
    }

    pub fn parse_expression(&mut self) -> Result<Box<dyn Expr>, ParseError> {
        logln(
            LogLevel::Trace,
            &format!(
                "parse_expression start cur={:?} peek={:?}",
                self.cur, self.peek
            ),
        );
        if let Token::Ident(name) = &self.cur
            && self.peek == Token::Arrow
        {
            let params = vec![name.clone()];
            self.bump();
            self.bump();
            let body: Rc<[Box<dyn Stmt>]> = if self.cur == Token::LBrace {
                Rc::from(self.parse_block_body()?)
            } else {
                let expr = self.parse_expression()?;
                Rc::new([Box::new(expr::Return {
                    expr: Some(expr),
                    rtype: expr::ReturnType::Return,
                })])
            };
            return Ok(Box::new(expr::FunctionDecl {
                name: "anonymous function",
                params,
                body,
                generator: false,
                insert: false,
            }));
        }
        let expr = expr::Operator::parse(self)?;
        if let Token::Assign(t) = &self.cur {
            let t = t.clone();
            self.bump();
            let rhs = if let Some(op) = t {
                Box::new(expr::Operator {
                    left: expr.duplicate_expr(),
                    op,
                    right: self.parse_expression()?,
                })
            } else {
                self.parse_expression()?
            };
            Ok(Box::new(expr::Assign {
                target: expr,
                value: rhs,
            }))
        } else {
            Ok(expr)
        }
    }

    pub fn parse_primary(&mut self) -> Result<Box<dyn Expr>, ParseError> {
        logln(
            LogLevel::Trace,
            &format!("parse_primary cur={:?}", self.cur),
        );
        match &self.cur {
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                Ok(Box::new(expr::Identifier { name }))
            }
            Token::BigInt(n) => {
                let value = *n;
                self.bump();
                Ok(Box::new(expr::ConstBigInt { num: value }))
            }
            Token::Number(n) => {
                let value = *n;
                self.bump();
                Ok(Box::new(expr::ConstNumber { num: value }))
            }
            Token::TemplateStart => {
                self.bump();
                Ok(Box::new(expr::TemplateLiteral::parse(self)?))
            }
            Token::Str(s) => {
                let value = s.clone();
                self.bump();
                Ok(Box::new(expr::ConstString { s: value }))
            }
            Token::True => {
                self.bump();
                Ok(Box::new(expr::ConstBoolean { b: true }))
            }
            Token::False => {
                self.bump();
                Ok(Box::new(expr::ConstBoolean { b: false }))
            }
            Token::LBrace => {
                self.bump();
                let mut properties = Vec::new();
                while self.cur != Token::RBrace && self.cur != Token::Eof {
                    let key: Box<dyn Expr> = match &self.cur {
                        Token::Ident(value) | Token::Str(value) => {
                            let key = Box::new(expr::ConstString { s: value.clone() });
                            self.bump();
                            key
                        }
                        Token::BigInt(n) => {
                            let key = Box::new(expr::ConstBigInt { num: *n });
                            self.bump();
                            key
                        }
                        Token::Number(n) => {
                            let key = Box::new(expr::ConstNumber { num: *n });
                            self.bump();
                            key
                        }
                        _ => {
                            return Err(ParseError(format!(
                                "unexpected object key: {:?}",
                                self.cur
                            )));
                        }
                    };
                    if self.cur != Token::Colon {
                        return Err(ParseError(format!(
                            "expected ':' in object literal, got {:?}",
                            self.cur
                        )));
                    }
                    self.bump();
                    let value = self.parse_expression()?;
                    properties.push((key, value));
                    if self.cur == Token::Comma {
                        self.bump();
                    }
                }
                if self.cur != Token::RBrace {
                    return Err(ParseError("expected '}' at end of object literal".into()));
                }
                // consume '}' so callers see the token after the object literal
                self.bump();
                Ok(Box::new(expr::Object { properties }))
            }
            Token::LBracket => {
                self.bump();
                let mut elements = Vec::new();
                while self.cur != Token::RBracket && self.cur != Token::Eof {
                    elements.push(self.parse_expression()?);
                    if self.cur == Token::Comma {
                        self.bump();
                    }
                }
                if self.cur != Token::RBracket {
                    return Err(ParseError("expected ']' at end of array literal".into()));
                }
                self.bump();
                Ok(Box::new(expr::Array { elems: elements }))
            }
            Token::New => {
                self.bump();
                let mut constructor = self.parse_primary()?;
                loop {
                    if self.cur == Token::Dot {
                        self.bump();
                        if let Token::Ident(name) = &self.cur {
                            let prop = name.clone();
                            self.bump();
                            constructor = Box::new(expr::Member {
                                object: constructor,
                                property: Box::new(expr::ConstString { s: prop }),
                            });
                            continue;
                        } else {
                            return Err(ParseError(format!(
                                "expected property name after '.', got {:?}",
                                self.cur
                            )));
                        }
                    }
                    if self.cur == Token::LBracket {
                        self.bump();
                        let index = self.parse_expression()?;
                        if self.cur != Token::RBracket {
                            return Err(ParseError("expected ']'".into()));
                        }
                        self.bump();
                        constructor = Box::new(expr::Member {
                            object: constructor,
                            property: index,
                        });
                        continue;
                    }
                    break;
                }
                let mut args = Vec::new();
                if self.cur == Token::LParen {
                    self.bump();
                    while self.cur != Token::RParen && self.cur != Token::Eof {
                        args.push(self.parse_expression()?);
                        if self.cur == Token::Comma {
                            self.bump();
                        }
                    }
                    if self.cur == Token::RParen {
                        self.bump();
                    }
                }
                Ok(Box::new(expr::New { constructor, args }))
            }
            Token::Function => {
                // simple function expression: function name? (params) { ... }
                self.bump();
                Ok(Box::new(expr::FunctionDecl::parse(self, false)?))
            }
            Token::LParen => {
                // grouping or call
                self.bump();
                let expr = self.parse_expression()?;
                while self.cur != Token::RParen && self.cur != Token::Eof {
                    self.bump();
                }
                if self.cur == Token::RParen {
                    self.bump();
                }
                Ok(expr)
            }
            _ => {
                let msg = format!("unexpected token in primary: {:?}", self.cur);
                logln(LogLevel::Error, &msg);
                Err(ParseError(msg))
            }
        }
    }

    pub fn parse_call_or_primary(&mut self) -> Result<Box<dyn Expr>, ParseError> {
        logln(
            LogLevel::Trace,
            &format!("parse_call_or_primary cur={:?}", self.cur),
        );
        let mut expr = self.parse_primary()?;
        loop {
            match self.cur {
                Token::Dot => {
                    self.bump();
                    if let Token::Ident(name) = &self.cur {
                        let prop = name.clone();
                        self.bump();
                        expr = Box::new(expr::Member {
                            object: expr,
                            property: Box::new(expr::ConstString { s: prop }),
                        });
                    } else {
                        return Err(ParseError(format!(
                            "expected property name after '.', got {:?}",
                            self.cur
                        )));
                    }
                }
                Token::LBracket => {
                    self.bump();
                    let index = self.parse_expression()?;
                    if self.cur != Token::RBracket {
                        return Err(ParseError("expected ']'".into()));
                    }
                    self.bump();
                    expr = Box::new(expr::Member {
                        object: expr,
                        property: index,
                    });
                }
                Token::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    while self.cur != Token::RParen && self.cur != Token::Eof {
                        let arg = self.parse_expression()?;
                        args.push(arg);
                        if self.cur == Token::Comma {
                            self.bump();
                        }
                    }
                    expr = Box::new(expr::Call { func: expr, args });
                    if self.cur == Token::RParen {
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
        Ok(expr)
    }
}

pub fn parse(input: &str) -> Result<Program, ParseError> {
    let mut p = Parser::new(input);
    p.parse_program()
}
