#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Number(f64),
    Str(String),
    TemplateStart,
    TemplateEnd,
    TemplateString(String),
    TemplateExprStart,
    True,
    False,
    Function,
    Class,
    Return,
    Let,
    Const,
    Var,
    New,
    While,
    For,
    Do,
    Break,
    Continue,
    If,
    Else,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Dot,
    Assign,
    Colon,

    Plus,
    PlusPlus,
    Minus,
    Star,
    Slash,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Arrow,

    Eof,
}

pub struct Lexer<'a> {
    chars: std::str::Chars<'a>,
    peeked: Option<char>,
    pending: Option<Token>,
    template_mode: bool,
    in_template_expr: bool,
    template_expr_depth: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let peeked = chars.next();
        Self {
            chars,
            peeked,
            pending: None,
            template_mode: false,
            in_template_expr: false,
            template_expr_depth: 0,
        }
    }

    fn bump(&mut self) -> Option<char> {
        let cur = self.peeked;
        self.peeked = self.chars.next();
        cur
    }

    const fn peek(&self) -> Option<char> { self.peeked }

    fn eat_while<F>(&mut self, mut f: F) -> String where F: FnMut(char) -> bool {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if f(c) {
                s.push(c);
                self.bump();
            } else { break }
        }
        s
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.pending.take() {
            return token;
        }

        if self.template_mode {
            let mut literal = String::new();
            while let Some(ch) = self.peek() {
                if ch == '`' {
                    self.bump();
                    self.template_mode = false;
                    if !literal.is_empty() {
                        self.pending = Some(Token::TemplateEnd);
                        return Token::TemplateString(literal);
                    }
                    return Token::TemplateEnd;
                }
                if ch == '$' {
                    let mut clone = self.chars.clone();
                    if let Some('{') = clone.next() {
                        self.bump();
                        self.bump();
                        self.template_mode = false;
                        self.in_template_expr = true;
                        self.template_expr_depth = 0;
                        if !literal.is_empty() {
                            self.pending = Some(Token::TemplateExprStart);
                            return Token::TemplateString(literal);
                        }
                        return Token::TemplateExprStart;
                    }
                }
                if ch == '\\' {
                    self.bump();
                    if let Some(escaped) = self.bump() {
                        literal.push(escaped);
                    }
                    continue;
                }
                self.bump();
                literal.push(ch);
            }
            self.template_mode = false;
            return Token::TemplateString(literal);
        }

        while let Some(c) = self.peek() {
            if self.in_template_expr {
                if c == '{' {
                    self.bump();
                    self.template_expr_depth += 1;
                    return Token::LBrace;
                }
                if c == '}' {
                    self.bump();
                    if self.template_expr_depth == 0 {
                        self.in_template_expr = false;
                        self.template_mode = true;
                    } else {
                        self.template_expr_depth -= 1;
                    }
                    return Token::RBrace;
                }
            }
            match c {
                c if c.is_whitespace() => { self.bump(); continue; }
                '/' => {
                    self.bump();
                    if self.peek() == Some('/') {
                        // line comment
                        self.bump();
                        self.eat_while(|ch| ch != '\n');
                        continue;
                    } else {
                        return Token::Slash;
                    }
                }
                '(' => { self.bump(); return Token::LParen }
                ')' => { self.bump(); return Token::RParen }
                '{' => { self.bump(); return Token::LBrace }
                '}' => { self.bump(); return Token::RBrace }
                '[' => { self.bump(); return Token::LBracket }
                ']' => { self.bump(); return Token::RBracket }
                ',' => { self.bump(); return Token::Comma }
                ';' => { self.bump(); return Token::Semicolon }
                '.' => { self.bump(); return Token::Dot }
                ':' => { self.bump(); return Token::Colon }
                '=' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        return Token::Arrow;
                    }
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Eq;
                    } else {
                        return Token::Assign;
                    }
                }
                '!' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::NotEq;
                    } else {
                        // For now, skip unknown '!'
                        continue;
                    }
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::LtEq;
                    } else {
                        return Token::Lt;
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::GtEq;
                    } else {
                        return Token::Gt;
                    }
                }
                '+' => {
                    self.bump();
                    if self.peek() == Some('+') {
                        self.bump();
                        return Token::PlusPlus;
                    }
                    return Token::Plus
                }
                '-' => { self.bump(); return Token::Minus }
                '*' => { self.bump(); return Token::Star }
                '`' => {
                    self.bump();
                    self.template_mode = true;
                    return Token::TemplateStart;
                }
                '"' | '\'' => {
                    let quote = self.bump().expect("Lexer bump should return the opening quote");
                    let mut s = String::new();
                    while let Some(ch) = self.peek() {
                        self.bump();
                        if ch == quote { break }
                        s.push(ch);
                    }
                    return Token::Str(s);
                }
                c if c.is_ascii_digit() => {
                    let s = self.eat_while(|ch| ch.is_ascii_digit() || ch == '.');
                    if let Ok(n) = s.parse::<f64>() {
                        return Token::Number(n);
                    } else { continue }
                }
                c if is_ident_start(c) => {
                    let s = self.eat_while(is_ident_continue);
                    return match s.as_str() {
                        "function" => Token::Function,
                        "class" => Token::Class,
                        "return" => Token::Return,
                        "let" => Token::Let,
                        "const" => Token::Const,
                        "var" => Token::Var,
                        "new" => Token::New,
                        "true" => Token::True,
                        "false" => Token::False,
                        "while" => Token::While,
                        "for" => Token::For,
                        "do" => Token::Do,
                        "break" => Token::Break,
                        "continue" => Token::Continue,
                        "if" => Token::If,
                        "else" => Token::Else,
                        _ => Token::Ident(s),
                    }
                }
                _ => { self.bump(); }
            }
        }
        Token::Eof
    }
}

const fn is_ident_start(c: char) -> bool { c.is_ascii_alphabetic() || c == '_' }
const fn is_ident_continue(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' }
