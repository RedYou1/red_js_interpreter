#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    BigInt(i64),
    Number(f64),
    Str(String),
    TemplateStart,
    TemplateEnd,
    TemplateString(String),
    TemplateExprStart,
    True,
    False,
    Undefined,
    Null,
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
    Yield,
    If,
    Else,
    Switch,
    Case,
    Default,
    In,
    Of,
    Typeof,
    Void,
    Delete,
    InstanceOf,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Dot,
    Spread,
    Assign(Option<BinaryOp>),
    QMark,
    Nullish,
    Colon,

    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    Mod,
    Eq,
    NotEq,
    Not,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Arrow,
    BitNot,

    And,
    Or,
    XOr,
    ShiftL,
    ShiftR,

    Try,
    Catch,
    Finally,
    Throw,

    Regex(String, String),

    Eof,
}

impl Token {
    pub fn from_keyword(value: String) -> Self {
        match value.as_str() {
            "function" => Token::Function,
            "class" => Token::Class,
            "return" => Token::Return,
            "let" => Token::Let,
            "const" => Token::Const,
            "var" => Token::Var,
            "new" => Token::New,
            "true" => Token::True,
            "false" => Token::False,
            "undefined" => Token::Undefined,
            "null" => Token::Null,
            "while" => Token::While,
            "for" => Token::For,
            "do" => Token::Do,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "yield" => Token::Yield,
            "if" => Token::If,
            "else" => Token::Else,
            "switch" => Token::Switch,
            "case" => Token::Case,
            "default" => Token::Default,
            "in" => Token::In,
            "of" => Token::Of,
            "typeof" => Token::Typeof,
            "void" => Token::Void,
            "delete" => Token::Delete,
            "instanceof" => Token::InstanceOf,
            "try" => Token::Try,
            "catch" => Token::Catch,
            "finally" => Token::Finally,
            "throw" => Token::Throw,
            _ => Token::Ident(value),
        }
    }

    pub fn as_keyword(&self) -> &str {
        match self {
            Token::Function => "function",
            Token::Class => "class",
            Token::Return => "return",
            Token::Let => "let",
            Token::Const => "const",
            Token::Var => "var",
            Token::New => "new",
            Token::True => "true",
            Token::False => "false",
            Token::Undefined => "undefined",
            Token::Null => "null",
            Token::While => "while",
            Token::For => "for",
            Token::Do => "do",
            Token::Break => "break",
            Token::Continue => "continue",
            Token::Yield => "yield",
            Token::If => "if",
            Token::Else => "else",
            Token::Switch => "switch",
            Token::Case => "case",
            Token::Default => "default",
            Token::In => "in",
            Token::Of => "of",
            Token::Typeof => "typeof",
            Token::Void => "void",
            Token::Delete => "delete",
            Token::InstanceOf => "instanceof",
            Token::Spread => "...",
            Token::Nullish => "??",
            Token::BitNot => "~",
            Token::Try => "try",
            Token::Catch => "catch",
            Token::Finally => "finally",
            Token::Throw => "throw",
            Token::Ident(value) => value.as_str(),
            e => panic!("into_keyword not supported {e:?}"),
        }
    }
}

use crate::parser::expr::BinaryOp;

#[derive(Clone, Copy)]
struct TemplateFrame {
    in_expression: bool,
    expression_depth: usize,
}

pub struct Lexer<'a> {
    chars: std::str::Chars<'a>,
    peeked: Option<char>,
    pending: Option<Token>,
    template_stack: Vec<TemplateFrame>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let peeked = chars.next();
        Self {
            chars,
            peeked,
            pending: None,
            template_stack: Vec::new(),
        }
    }

    fn bump(&mut self) -> Option<char> {
        let cur = self.peeked;
        self.peeked = self.chars.next();
        cur
    }

    pub const fn peek(&self) -> Option<char> {
        self.peeked
    }

    fn eat_while<F>(&mut self, mut f: F) -> String
    where
        F: FnMut(char) -> bool,
    {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if f(c) {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        s
    }

    fn read_escaped_char(&mut self) -> Option<char> {
        self.bump()?;
        self.bump().map(|escaped| match escaped {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            '`' => '`',
            other => other,
        })
    }

    fn starts_with_ellipsis(&self) -> bool {
        if self.peek() != Some('.') {
            return false;
        }
        let mut remaining = self.chars.clone();
        remaining.next() == Some('.') && remaining.next() == Some('.')
    }

    fn read_number(&mut self) -> Option<(String, f64)> {
        let mut number = String::new();
        if self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            number.push_str(&self.eat_while(|character| character.is_ascii_digit()));
            if self.peek() == Some('.') && !self.starts_with_ellipsis() {
                number.push(self.bump()?);
                number.push_str(&self.eat_while(|character| character.is_ascii_digit()));
            }
        } else if self.peek() == Some('.') && !self.starts_with_ellipsis() {
            let mut remaining = self.chars.clone();
            if !remaining
                .next()
                .is_some_and(|character| character.is_ascii_digit())
            {
                return None;
            }
            number.push(self.bump()?);
            number.push_str(&self.eat_while(|character| character.is_ascii_digit()));
        } else {
            return None;
        }

        let power = if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            let negative = matches!(self.peek(), Some('-'));
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            let exponent = self
                .eat_while(|character| character.is_ascii_digit())
                .parse::<u32>()
                .unwrap();
            10.0_f64.powf(exponent as f64 * if negative { -1.0 } else { 1.0 })
        } else {
            1.0_f64
        };
        if self.peek() == Some('n') {
            self.bump();
        }
        Some((number, power))
    }

    pub fn next_token(&mut self, prev: &[Token]) -> Token {
        // logln(
        //     LogLevel::Trace,
        //     &format!(
        //         "Lexer::next_token start peek={:?} template_mode={} in_template_expr={} template_expr_depth={}",
        //         self.peek(),
        //         self.template_mode,
        //         self.in_template_expr,
        //         self.template_expr_depth
        //     ),
        // );
        if let Some(token) = self.pending.take() {
            return token;
        }

        if self
            .template_stack
            .last()
            .is_some_and(|frame| !frame.in_expression)
        {
            let mut literal = String::new();
            while let Some(ch) = self.peek() {
                if ch == '`' {
                    self.bump();
                    self.template_stack.pop();
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
                        if let Some(frame) = self.template_stack.last_mut() {
                            frame.in_expression = true;
                            frame.expression_depth = 0;
                        }
                        if !literal.is_empty() {
                            self.pending = Some(Token::TemplateExprStart);
                            return Token::TemplateString(literal);
                        }
                        return Token::TemplateExprStart;
                    }
                }
                if ch == '\\' {
                    if let Some(escaped) = self.read_escaped_char() {
                        literal.push(escaped);
                    }
                    continue;
                }
                self.bump();
                literal.push(ch);
            }
            self.template_stack.pop();
            return Token::TemplateString(literal);
        }

        while let Some(c) = self.peek() {
            if self
                .template_stack
                .last()
                .is_some_and(|frame| frame.in_expression)
            {
                if c == '{' {
                    self.bump();
                    if let Some(frame) = self.template_stack.last_mut() {
                        frame.expression_depth += 1;
                    }
                    return Token::LBrace;
                }
                if c == '}' {
                    self.bump();
                    if let Some(frame) = self.template_stack.last_mut() {
                        if frame.expression_depth == 0 {
                            frame.in_expression = false;
                        } else {
                            frame.expression_depth -= 1;
                        }
                    }
                    return Token::RBrace;
                }
            }
            match c {
                c if c.is_whitespace() => {
                    self.bump();
                    continue;
                }
                '/' => {
                    self.bump();
                    if self.peek() == Some('/') {
                        // line comment
                        self.bump();
                        self.eat_while(|ch| ch != '\n');
                        continue;
                    }
                    if self.peek() == Some('*') {
                        self.bump();
                        loop {
                            let a = self.peek() == Some('*');
                            self.bump();
                            if a {
                                let a = self.peek() == Some('/');
                                self.bump();
                                if a {
                                    break;
                                }
                            }
                        }
                        continue;
                    }
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::Div));
                    }

                    if let Some(last) = prev.last() {
                        if matches!(
                            last,
                            Token::Ident(_)
                                | Token::Number(_)
                                | Token::BigInt(_)
                                | Token::Str(_)
                                | Token::RParen
                                | Token::RBrace
                                | Token::RBracket
                        ) {
                            return Token::Slash;
                        } else {
                            let mut s = String::new();
                            let mut in_class = false;
                            while let Some(ch) = self.peek() {
                                if ch == '\\' {
                                    self.bump();
                                    s.push('\\');
                                    if let Some(escaped) = self.bump() {
                                        s.push(escaped);
                                    }
                                    continue;
                                }
                                self.bump();
                                if ch == '[' {
                                    in_class = true;
                                } else if ch == ']' {
                                    in_class = false;
                                } else if ch == '/' && !in_class {
                                    break;
                                }
                                s.push(ch);
                            }
                            let flags = self.eat_while(|flag| flag.is_ascii_alphabetic());
                            return Token::Regex(s, flags);
                        }
                    }

                    return Token::Slash;
                }
                '&' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::BinAnd));
                    }
                    return Token::And;
                }
                '|' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::BinOr));
                    }
                    return Token::Or;
                }
                '^' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::XOr));
                    }
                    return Token::XOr;
                }
                '(' => {
                    self.bump();
                    return Token::LParen;
                }
                ')' => {
                    self.bump();
                    return Token::RParen;
                }
                '{' => {
                    self.bump();
                    return Token::LBrace;
                }
                '}' => {
                    self.bump();
                    return Token::RBrace;
                }
                '[' => {
                    self.bump();
                    return Token::LBracket;
                }
                ']' => {
                    self.bump();
                    return Token::RBracket;
                }
                ',' => {
                    self.bump();
                    return Token::Comma;
                }
                ';' => {
                    self.bump();
                    return Token::Semicolon;
                }
                ':' => {
                    self.bump();
                    return Token::Colon;
                }
                '=' => {
                    self.bump();
                    if self.peek() == Some('>') {
                        self.bump();
                        return Token::Arrow;
                    }
                    if self.peek() == Some('=') {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                        }
                        return Token::Eq;
                    } else {
                        return Token::Assign(None);
                    }
                }
                '!' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                        }
                        return Token::NotEq;
                    } else {
                        return Token::Not;
                    }
                }
                '~' => {
                    self.bump();
                    return Token::BitNot;
                }
                '?' => {
                    self.bump();
                    if self.peek() == Some('?') {
                        self.bump();
                        return Token::Nullish;
                    }
                    return Token::QMark;
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::LtEq;
                    } else if self.peek() == Some('<') {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                            return Token::Assign(Some(BinaryOp::ShiftL));
                        }
                        return Token::ShiftL;
                    } else {
                        return Token::Lt;
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::GtEq;
                    } else if self.peek() == Some('>') {
                        self.bump();
                        if self.peek() == Some('>') {
                            self.bump();
                        }
                        if self.peek() == Some('=') {
                            self.bump();
                            return Token::Assign(Some(BinaryOp::ShiftR));
                        }
                        return Token::ShiftR;
                    } else {
                        return Token::Gt;
                    }
                }
                '.' if self.starts_with_ellipsis() => {
                    self.bump();
                    self.bump();
                    self.bump();
                    return Token::Spread;
                }
                '.' if !self
                    .chars
                    .clone()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit()) =>
                {
                    self.bump();
                    return Token::Dot;
                }
                '%' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::Mod));
                    }
                    return Token::Mod;
                }
                '+' => {
                    self.bump();
                    if self.peek() == Some('+') {
                        self.bump();
                        return Token::PlusPlus;
                    }
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::Add));
                    }
                    return Token::Plus;
                }
                '-' => {
                    self.bump();
                    if self.peek() == Some('-') {
                        self.bump();
                        return Token::MinusMinus;
                    }
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::Sub));
                    }
                    return Token::Minus;
                }
                '*' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        return Token::Assign(Some(BinaryOp::Mul));
                    }
                    return Token::Star;
                }
                '`' => {
                    self.bump();
                    self.template_stack.push(TemplateFrame {
                        in_expression: false,
                        expression_depth: 0,
                    });
                    return Token::TemplateStart;
                }
                '"' | '\'' => {
                    let quote = self
                        .bump()
                        .expect("Lexer bump should return the opening quote");
                    let mut s = String::new();
                    while let Some(ch) = self.peek() {
                        if ch == '\\' {
                            if let Some(escaped) = self.read_escaped_char() {
                                s.push(escaped);
                            }
                            continue;
                        }
                        self.bump();
                        if ch == quote {
                            break;
                        }
                        s.push(ch);
                    }
                    return Token::Str(s);
                }
                c if is_ident_start(c) => {
                    return Token::from_keyword(self.eat_while(is_ident_continue));
                }
                _ if let Some((s, power)) = self.read_number() => {
                    if let Ok(n) = s.parse::<i64>() {
                        let n = n as f64 * power;
                        if ((n as i64) as f64) == n {
                            return Token::BigInt(n as i64);
                        } else {
                            return Token::Number(n);
                        }
                    } else if let Ok(n) = s.parse::<f64>() {
                        let n = n * power;
                        if ((n as i64) as f64) == n {
                            return Token::BigInt(n as i64);
                        } else {
                            return Token::Number(n);
                        }
                    } else {
                        panic!("number not tokenized: {s:?}");
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
        Token::Eof
    }
}

const fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
const fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Token};

    #[test]
    fn lexes_decimal_exponents() {
        for (source, expected) in [
            ("1e4", Token::BigInt(10_000)),
            ("1E4", Token::BigInt(10_000)),
            ("1e-4", Token::Number(0.0001)),
            ("1E+4", Token::BigInt(10_000)),
            ("1.5e2", Token::BigInt(150)),
            ("4n", Token::BigInt(4)),
        ] {
            let mut lexer = Lexer::new(source);
            assert_eq!(lexer.next_token(&[]), expected);
            assert_eq!(lexer.next_token(&[expected]), Token::Eof);
        }
    }

    #[test]
    fn separates_numeric_literal_from_ellipsis() {
        let mut lexer = Lexer::new("0...");
        assert_eq!(lexer.next_token(&[]), Token::BigInt(0));
        assert_eq!(lexer.next_token(&[]), Token::Spread);
        assert_eq!(lexer.next_token(&[]), Token::Eof);
    }

    #[test]
    fn lexes_fraction_without_leading_zero() {
        let mut lexer = Lexer::new(".5");
        assert_eq!(lexer.next_token(&[]), Token::Number(0.5));
        assert_eq!(lexer.next_token(&[]), Token::Eof);
    }

    #[test]
    fn lexes_member_access_dot() {
        let mut lexer = Lexer::new("console.log");
        assert_eq!(lexer.next_token(&[]), Token::Ident("console".to_owned()));
        assert_eq!(lexer.next_token(&[]), Token::Dot);
        assert_eq!(lexer.next_token(&[]), Token::Ident("log".to_owned()));
        assert_eq!(lexer.next_token(&[]), Token::Eof);
    }
}
