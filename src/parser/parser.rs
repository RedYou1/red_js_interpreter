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

    pub fn parse_function_body(&mut self) -> Result<Vec<Box<dyn Stmt>>, ParseError> {
        if self.cur != Token::LBrace {
            return Err(ParseError("expected '{'".into()));
        }
        self.bump();
        let mut body: Vec<Box<dyn Stmt>> = Vec::new();
        while self.cur != Token::RBrace && self.cur != Token::Eof {
            match &self.cur {
                Token::Return => {
                    self.bump();
                    let expr = self.parse_expression()?;
                    body.push(Box::new(expr::Return {
                        expr: Some(expr),
                        rtype: expr::ReturnType::Return,
                    }));
                    if self.cur == Token::Semicolon {
                        self.bump();
                    }
                }
                Token::Semicolon => {
                    self.bump();
                }
                _ => {
                    let expr = self.parse_expression()?;
                    body.push(expr);
                    if self.cur == Token::Semicolon {
                        self.bump();
                    }
                }
            }
        }
        if self.cur == Token::RBrace {
            self.bump();
        }
        Ok(body)
    }

    pub fn parse_block_body(&mut self) -> Result<Vec<Box<dyn Stmt>>, ParseError> {
        if self.cur != Token::LBrace {
            return Err(ParseError("expected '{'".into()));
        }
        self.bump();
        let mut body = Vec::new();
        while self.cur != Token::RBrace && self.cur != Token::Eof {
            if let Some(stmt) = self.parse_statement()? {
                body.push(stmt);
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
                self.bump(); // consume 'function'
                let name = self.expect_ident()?.leak();
                logln(
                    LogLevel::Info,
                    &format!("parse_statement function name={}", name),
                );
                // Expect LParen
                self.skip_to(Token::LParen);
                self.expect_and_bump(Token::LParen, "expected '('")?;
                let params = self.parse_param_list()?;
                self.skip_to(Token::LBrace);
                let body = self.parse_function_body()?;
                Ok(Some(Box::new(expr::FunctionDecl {
                    name,
                    params,
                    body,
                    generator: false,
                    insert: true,
                })))
            }
            Token::Class => {
                self.bump();
                let name = if let Token::Ident(_) = &self.cur {
                    self.expect_ident()?.leak()
                } else {
                    return Err(ParseError("expected class name".into()));
                };
                logln(
                    LogLevel::Info,
                    &format!("parse_statement class name={}", name),
                );
                let super_class = if let Token::Ident(ident) = &self.cur {
                    if ident == "extends" {
                        self.bump();
                        Some(self.parse_call_or_primary()?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if self.cur != Token::LBrace {
                    return Err(ParseError("expected '{' after class name".into()));
                }
                self.bump();
                let mut methods = Vec::new();
                while self.cur != Token::RBrace && self.cur != Token::Eof {
                    if let Token::Ident(_) = &self.cur {
                        let method_name = self.expect_ident()?.leak();
                        self.expect_and_bump(Token::LParen, "expected '(' in method")?;
                        let params = self.parse_param_list()?;
                        let body = self.parse_function_body()?;
                        methods.push(expr::FunctionDecl {
                            name: method_name,
                            params,
                            body,
                            generator: false,
                            insert: false,
                        });
                        continue;
                    }
                    self.bump();
                }
                let class = expr::ClassDecl {
                    name,
                    super_class,
                    methods: Rc::from(methods),
                };
                if self.cur == Token::RBrace {
                    self.bump();
                }
                Ok(Some(Box::new(class)))
            }
            Token::Let | Token::Const | Token::Var => {
                self.bump();
                let name = self.expect_ident()?;
                logln(
                    LogLevel::Info,
                    &format!("parse_statement variable declaration name={}", name),
                );
                let initializer = if self.cur == Token::Assign {
                    self.bump();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Box::new(expr::VarDecl { name, initializer })))
            }
            Token::If => {
                self.bump();
                logln(LogLevel::Info, "parse_statement if statement");
                if self.cur != Token::LParen {
                    return Err(ParseError("expected '(' after 'if'".into()));
                }
                self.bump();
                let condition = self.parse_expression()?;
                if self.cur != Token::RParen {
                    return Err(ParseError("expected ')' after if condition".into()));
                }
                self.bump();

                if self.cur != Token::LBrace {
                    return Err(ParseError("expected '{' after if".into()));
                }

                let mut consequent = self.parse_block_body()?;
                if consequent.len() != 1 {
                    return Err(ParseError("Multiple bloc for same if".to_owned()));
                };

                let mut blocks = vec![(condition, consequent.pop().unwrap())];

                if self.cur == Token::Else {
                    self.bump();
                    let mut consequent = self.parse_block_body()?;
                    if consequent.len() != 1 {
                        return Err(ParseError("Multiple bloc for same if".to_owned()));
                    };
                    blocks.push((
                        Box::new(expr::ConstBoolean { b: true }),
                        consequent.pop().unwrap(),
                    ));
                }

                Ok(Some(Box::new(stmt::IfStmt { blocks })))
            }
            Token::While => {
                self.bump();
                logln(LogLevel::Info, "parse_statement while loop");
                if self.cur != Token::LParen {
                    return Err(ParseError("expected '(' after 'while'".into()));
                }
                self.bump();
                let condition = self.parse_expression()?;
                if self.cur != Token::RParen {
                    return Err(ParseError("expected ')' after while condition".into()));
                }
                self.bump();

                if self.cur != Token::LBrace {
                    return Err(ParseError("expected '{' for while body".into()));
                }
                let body = self.parse_block_body()?;

                Ok(Some(Box::new(stmt::LoopStmt {
                    init: None,
                    condition,
                    body,
                    update: None,
                    do_first: false,
                })))
            }
            Token::For => {
                self.bump();
                logln(LogLevel::Info, "parse_statement for loop");
                if self.cur != Token::LParen {
                    return Err(ParseError("expected '(' after 'for'".into()));
                }
                self.bump();

                // Parse init
                let init: Option<Box<dyn Expr>> = if self.cur == Token::Semicolon {
                    None
                } else if self.cur == Token::Let
                    || self.cur == Token::Const
                    || self.cur == Token::Var
                {
                    self.bump();
                    let name = self.expect_ident()?;
                    let initializer = if self.cur == Token::Assign {
                        self.bump();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    if self.cur == Token::Semicolon {
                        self.bump();
                    }
                    Some(Box::new(expr::VarDecl { name, initializer }))
                } else {
                    let expr = self.parse_expression()?;
                    if self.cur == Token::Semicolon {
                        self.bump();
                    }
                    Some(expr)
                };

                // Parse condition
                let condition: Box<dyn Expr> = if self.cur == Token::Semicolon {
                    Box::new(expr::ConstBoolean { b: true })
                } else {
                    self.parse_expression()?
                };
                if self.cur == Token::Semicolon {
                    self.bump();
                }

                // Parse update
                let update = if self.cur == Token::RParen {
                    None
                } else {
                    Some(self.parse_expression()?)
                };

                if self.cur != Token::RParen {
                    return Err(ParseError("expected ')' after for clauses".into()));
                }
                self.bump();

                if self.cur != Token::LBrace {
                    return Err(ParseError("expected '{' for for body".into()));
                }
                let body = self.parse_block_body()?;

                Ok(Some(Box::new(stmt::LoopStmt {
                    init,
                    condition,
                    update,
                    body,
                    do_first: false,
                })))
            }
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
                let condition = self.parse_expression()?;
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
            Token::Break => {
                self.bump();
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Box::new(expr::Return {
                    expr: None,
                    rtype: expr::ReturnType::Break,
                })))
            }
            Token::Continue => {
                self.bump();
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Box::new(expr::Return {
                    expr: None,
                    rtype: expr::ReturnType::Continue,
                })))
            }
            Token::Return => {
                self.bump();
                logln(LogLevel::Info, "parse_statement return statement");
                let expr = self.parse_expression()?;
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Box::new(expr::Return {
                    expr: Some(expr),
                    rtype: expr::ReturnType::Return,
                })))
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
            let body: Vec<Box<dyn Stmt>> = if self.cur == Token::LBrace {
                self.parse_block_body()?
            } else {
                let expr = self.parse_expression()?;
                vec![Box::new(expr::Return {
                    expr: Some(expr),
                    rtype: expr::ReturnType::Return,
                })]
            };
            return Ok(Box::new(expr::FunctionDecl {
                name: "anonymous function",
                params,
                body,
                generator: false,
                insert: false,
            }));
        }
        let expr = self.parse_binary(0)?;
        if self.cur == Token::Assign {
            self.bump();
            let rhs = self.parse_expression()?;
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
            Token::Number(n) => {
                let value = *n;
                self.bump();
                Ok(Box::new(expr::ConstNumber { num: value }))
            }
            Token::TemplateStart => {
                self.bump();
                let mut parts = Vec::new();
                while self.cur != Token::TemplateEnd && self.cur != Token::Eof {
                    match &self.cur {
                        Token::TemplateString(value) => {
                            parts.push(expr::TemplatePart::String(value.clone()));
                            self.bump();
                        }
                        Token::TemplateExprStart => {
                            self.bump();
                            let expr = self.parse_expression()?;
                            if self.cur != Token::RBrace {
                                return Err(ParseError(
                                    "expected '}' after template expression".into(),
                                ));
                            }
                            self.bump();
                            parts.push(expr::TemplatePart::Expr(expr));
                        }
                        _ => {
                            return Err(ParseError(format!(
                                "unexpected template token: {:?}",
                                self.cur
                            )));
                        }
                    }
                }
                if self.cur == Token::TemplateEnd {
                    self.bump();
                }
                Ok(Box::new(expr::TemplateLiteral { parts }))
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
                let name = if let Token::Ident(s) = &self.cur {
                    let name = s.clone().leak();
                    self.bump();
                    name
                } else {
                    "anonymous function"
                };
                self.skip_to(Token::LParen);
                self.expect_and_bump(Token::LParen, "expected '(' in function expr")?;
                let params = self.parse_param_list()?;
                self.skip_to(Token::LBrace);
                let body = self.parse_function_body()?;
                Ok(Box::new(expr::FunctionDecl {
                    name,
                    params,
                    body,
                    generator: false,
                    insert: false,
                }))
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
            if self.cur == Token::Dot {
                self.bump();
                if let Token::Ident(name) = &self.cur {
                    let prop = name.clone();
                    self.bump();
                    expr = Box::new(expr::Member {
                        object: expr,
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
                expr = Box::new(expr::Member {
                    object: expr,
                    property: index,
                });
                continue;
            }
            if self.cur == Token::LParen {
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
                continue;
            }
            if self.cur == Token::PlusPlus {
                self.bump();
                expr = Box::new(expr::Postfix { expr, inc: true });
                continue;
            }
            if self.cur == Token::MinusMinus {
                self.bump();
                expr = Box::new(expr::Postfix { expr, inc: false });
                continue;
            }
            break;
        }
        Ok(expr)
    }

    pub const fn precedence(&self, tok: &Token) -> Option<(u8, u8)> {
        match tok {
            Token::Eq | Token::NotEq | Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => {
                Some((8, 9))
            }
            Token::Plus | Token::Minus => Some((10, 11)),
            Token::Star | Token::Slash => Some((12, 13)),
            _ => None,
        }
    }

    pub fn parse_binary(&mut self, min_bp: u8) -> Result<Box<dyn Expr>, ParseError> {
        let mut lhs = self.parse_call_or_primary()?;
        // advance tokens for loop
        while let Some((l_bp, r_bp)) = self.precedence(&self.cur) {
            if l_bp < min_bp {
                break;
            }
            let op = match &self.cur {
                Token::Plus => expr::BinaryOp::Add,
                Token::Minus => expr::BinaryOp::Sub,
                Token::Star => expr::BinaryOp::Mul,
                Token::Slash => expr::BinaryOp::Div,
                Token::Eq => expr::BinaryOp::Eq,
                Token::NotEq => expr::BinaryOp::NotEq,
                Token::Lt => expr::BinaryOp::Lt,
                Token::Gt => expr::BinaryOp::Gt,
                Token::LtEq => expr::BinaryOp::LtEq,
                Token::GtEq => expr::BinaryOp::GtEq,
                _ => unreachable!(),
            };
            // consume operator
            self.bump();
            let rhs = self.parse_binary(r_bp)?;
            lhs = Box::new(expr::Operator {
                left: lhs,
                op,
                right: rhs,
            });
        }
        Ok(lhs)
    }
}

pub fn parse(input: &str) -> Result<Program, ParseError> {
    let mut p = Parser::new(input);
    p.parse_program()
}
