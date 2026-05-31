use crate::parser::ast::*;
use crate::parser::lexer::{Lexer, Token};

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
        Parser { lexer, cur, peek }
    }

    pub fn bump(&mut self) {
        self.cur = std::mem::replace(&mut self.peek, self.lexer.next_token());
    }

    pub fn expect_ident(&mut self) -> Result<String, ParseError> {
        match &self.cur {
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                Ok(name)
            }
            t => Err(ParseError(format!("expected identifier, got {:?}", t))),
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

    pub fn parse_function_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        if self.cur != Token::LBrace {
            return Err(ParseError("expected '{'".into()));
        }
        self.bump();
        let mut body = Vec::new();
        while self.cur != Token::RBrace && self.cur != Token::Eof {
            match &self.cur {
                Token::Return => {
                    self.bump();
                    let expr = self.parse_expression()?;
                    body.push(Stmt::Return(Some(expr)));
                    if self.cur == Token::Semicolon {
                        self.bump();
                    }
                }
                Token::Semicolon => {
                    self.bump();
                }
                _ => {
                    let expr = self.parse_expression()?;
                    body.push(Stmt::ExprStmt(expr));
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

    pub fn parse_block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
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
        Ok(Program { body })
    }

    pub fn parse_statement(&mut self) -> Result<Option<Stmt>, ParseError> {
        match &self.cur {
            Token::Function => {
                self.bump(); // consume 'function'
                let name = if let Token::Ident(_) = &self.cur {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                // after expect_ident, cur is set to Eof placeholder; restore peeked token by reusing bump
                // For simplicity, if we consumed the ident we bumped earlier; otherwise cur is still something else
                // Ensure paren
                if let Token::Ident(_) = &self.cur { /* handled */ }
                // If name was read via expect_ident it consumed cur, so set cur appropriately by using existing peek
                // Expect LParen
                self.skip_to(Token::LParen);
                self.expect_and_bump(Token::LParen, "expected '('")?;
                let params = self.parse_param_list()?;
                self.skip_to(Token::LBrace);
                let body = self.parse_function_body()?;
                let func = FunctionDecl { name, params, body };
                Ok(Some(Stmt::FunctionDecl(func)))
            }
            Token::Class => {
                self.bump();
                let name = if let Token::Ident(_) = &self.cur {
                    self.expect_ident()?
                } else {
                    return Err(ParseError("expected class name".into()));
                };
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
                        let method_name = self.expect_ident()?;
                        self.expect_and_bump(Token::LParen, "expected '(' in method")?;
                        let params = self.parse_param_list()?;
                        let body = self.parse_function_body()?;
                        methods.push(FunctionDecl {
                            name: Some(method_name),
                            params,
                            body,
                        });
                        continue;
                    }
                    self.bump();
                }
                let class = ClassDecl { name, super_class, methods };
                if self.cur == Token::RBrace {
                    self.bump();
                }
                Ok(Some(Stmt::ClassDecl(class)))
            }
            Token::Let | Token::Const | Token::Var => {
                self.bump();
                let name = self.expect_ident()?;
                let initializer = if self.cur == Token::Assign {
                    self.bump();
                    Some(self.parse_expression()? )
                } else {
                    None
                };
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Stmt::VarDecl(name, initializer)))
            }
            Token::If => {
                self.bump();
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
                let consequent = self.parse_block_body()?;
                
                let alternate = if self.cur == Token::Else {
                    self.bump();
                    if self.cur != Token::LBrace {
                        return Err(ParseError("expected '{' after else".into()));
                    }
                    self.bump();
                    let mut alt_body = Vec::new();
                    while self.cur != Token::RBrace && self.cur != Token::Eof {
                        if let Some(stmt) = self.parse_statement()? {
                            alt_body.push(stmt);
                        }
                    }
                    if self.cur == Token::RBrace {
                        self.bump();
                    }
                    Some(alt_body)
                } else {
                    None
                };
                
                Ok(Some(Stmt::If(IfStmt {
                    condition,
                    consequent,
                    alternate,
                })))
            }
            Token::While => {
                self.bump();
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
                
                Ok(Some(Stmt::While(WhileStmt { condition, body })))
            }
            Token::For => {
                self.bump();
                if self.cur != Token::LParen {
                    return Err(ParseError("expected '(' after 'for'".into()));
                }
                self.bump();
                
                // Parse init
                let init = if self.cur == Token::Semicolon {
                    None
                } else if self.cur == Token::Let || self.cur == Token::Const || self.cur == Token::Var {
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
                    Some(Box::new(Stmt::VarDecl(name, initializer)))
                } else {
                    let expr = self.parse_expression()?;
                    if self.cur == Token::Semicolon {
                        self.bump();
                    }
                    Some(Box::new(Stmt::ExprStmt(expr)))
                };
                
                // Parse condition
                let condition = if self.cur == Token::Semicolon {
                    None
                } else {
                    Some(self.parse_expression()?)
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
                
                Ok(Some(Stmt::For(ForStmt {
                    init,
                    condition,
                    update,
                    body,
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
                
                Ok(Some(Stmt::DoWhile(DoWhileStmt { body, condition })))
            }
            Token::Break => {
                self.bump();
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Stmt::Break))
            }
            Token::Continue => {
                self.bump();
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Stmt::Continue))
            }
            Token::Return => {
                self.bump();
                let expr = self.parse_expression()?;
                if self.cur == Token::Semicolon {
                    self.bump();
                }
                Ok(Some(Stmt::Return(Some(expr))))
            }
            Token::Semicolon => {
                self.bump();
                Ok(None)
            }
            Token::Eof => Ok(None),
            _ => {
                // expression statement
                let expr = self.parse_expression()?;
                Ok(Some(Stmt::ExprStmt(expr)))
            }
        }
    }

    pub fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_binary(0)?;
        if self.cur == Token::Assign {
            self.bump();
            let rhs = self.parse_expression()?;
            Ok(Expr::Assign(Box::new(expr), Box::new(rhs)))
        } else {
            Ok(expr)
        }
    }

    pub fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.cur {
            Token::Ident(s) => {
                let name = s.clone();
                self.bump();
                Ok(Expr::Identifier(name))
            }
            Token::Number(n) => {
                let value = *n;
                self.bump();
                Ok(Expr::Number(value))
            }
            Token::TemplateStart => {
                self.bump();
                let mut parts = Vec::new();
                while self.cur != Token::TemplateEnd && self.cur != Token::Eof {
                    match &self.cur {
                        Token::TemplateString(value) => {
                            parts.push(TemplatePart::String(value.clone()));
                            self.bump();
                        }
                        Token::TemplateExprStart => {
                            self.bump();
                            let expr = self.parse_expression()?;
                            if self.cur != Token::RBrace {
                                return Err(ParseError("expected '}' after template expression".into()));
                            }
                            self.bump();
                            parts.push(TemplatePart::Expr(Box::new(expr)));
                        }
                        _ => {
                            return Err(ParseError(format!("unexpected template token: {:?}", self.cur)));
                        }
                    }
                }
                if self.cur == Token::TemplateEnd {
                    self.bump();
                }
                Ok(Expr::TemplateLiteral(parts))
            }
            Token::Str(s) => {
                let value = s.clone();
                self.bump();
                Ok(Expr::String(value))
            }
            Token::True => {
                self.bump();
                Ok(Expr::Boolean(true))
            }
            Token::False => {
                self.bump();
                Ok(Expr::Boolean(false))
            }
            Token::LBrace => {
                self.bump();
                let mut properties = Vec::new();
                while self.cur != Token::RBrace && self.cur != Token::Eof {
                    let key = match &self.cur {
                        Token::Ident(name) => {
                            let key = Expr::String(name.clone());
                            self.bump();
                            key
                        }
                        Token::Str(value) => {
                            let key = Expr::String(value.clone());
                            self.bump();
                            key
                        }
                        Token::Number(n) => {
                            let key = Expr::Number(*n);
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
                        return Err(ParseError(format!("expected ':' in object literal, got {:?}", self.cur)));
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
                Ok(Expr::Object(properties))
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
                Ok(Expr::Array(elements))
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
                            constructor = Expr::Member(Box::new(constructor), prop);
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
                        constructor = Expr::Index(Box::new(constructor), Box::new(index));
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
                Ok(Expr::New(Box::new(constructor), args))
            }
            Token::Function => {
                // simple function expression: function (params) { ... }
                self.bump();
                let name = if let Token::Ident(s) = &self.cur {
                    Some(s.clone())
                } else {
                    None
                };
                if name.is_some() {
                    self.bump();
                }
                self.skip_to(Token::LParen);
                self.expect_and_bump(Token::LParen, "expected '(' in function expr")?;
                let params = self.parse_param_list()?;
                self.skip_to(Token::LBrace);
                let body = self.parse_function_body()?;
                let func = FunctionDecl { name, params, body };
                Ok(Expr::FunctionExpr(func))
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
            _ => Err(ParseError(format!(
                "unexpected token in primary: {:?}",
                self.cur
            ))),
        }
    }

    pub fn parse_call_or_primary(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.cur == Token::Arrow {
                self.bump();
                let params = match expr {
                    Expr::Identifier(name) => vec![name],
                    _ => return Err(ParseError("invalid arrow function parameter list".into())),
                };
                let body = if self.cur == Token::LBrace {
                    self.parse_block_body()?
                } else {
                    let expr = self.parse_expression()?;
                    vec![Stmt::Return(Some(expr))]
                };
                return Ok(Expr::FunctionExpr(FunctionDecl { name: None, params, body }));
            }
            if self.cur == Token::Dot {
                self.bump();
                if let Token::Ident(name) = &self.cur {
                    let prop = name.clone();
                    self.bump();
                    expr = Expr::Member(Box::new(expr), prop);
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
                expr = Expr::Index(Box::new(expr), Box::new(index));
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
                expr = Expr::Call(Box::new(expr), args);
                if self.cur == Token::RParen {
                    self.bump();
                }
                continue;
            }
            if self.cur == Token::Minus && self.peek == Token::Minus {
                self.bump();
                self.bump();
                expr = Expr::PostfixDec(Box::new(expr));
                continue;
            }
            if self.cur == Token::PlusPlus {
                self.bump();
                expr = Expr::PostfixInc(Box::new(expr));
                continue;
            }
            break;
        }
        Ok(expr)
    }

    pub const fn precedence(&self, tok: &Token) -> Option<(u8, u8)> {
        match tok {
            Token::Eq | Token::NotEq | Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => Some((8, 9)),
            Token::Plus | Token::Minus => Some((10, 11)),
            Token::Star | Token::Slash => Some((12, 13)),
            _ => None,
        }
    }

    pub fn parse_binary(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_call_or_primary()?;
        // advance tokens for loop
        while let Some((l_bp, r_bp)) = self.precedence(&self.cur) {
            if l_bp < min_bp {
                break;
            }
            let op = match &self.cur {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Eq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::NotEq,
                Token::Lt => BinaryOp::Lt,
                Token::Gt => BinaryOp::Gt,
                Token::LtEq => BinaryOp::LtEq,
                Token::GtEq => BinaryOp::GtEq,
                _ => unreachable!(),
            };
            // consume operator
            self.bump();
            let rhs = self.parse_binary(r_bp)?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs));
        }
        Ok(lhs)
    }
}

pub fn parse(input: &str) -> Result<Program, ParseError> {
    let mut p = Parser::new(input);
    p.parse_program()
}
