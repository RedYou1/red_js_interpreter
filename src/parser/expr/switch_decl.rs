use std::{cell::RefCell, rc::Rc};

use crate::{
    Code, CodeIndex, CodeResult, JsValue, LogLevel, Prototype, handle_return, inline_borrow, logln,
    parser::{
        expr::{self, Expr},
        lexer::Token,
        parser::Parser,
    },
    run_sub,
};

#[derive(Debug)]
pub struct SwitchExpr {
    pub value: Box<dyn Expr>,
    pub cases: Vec<(Option<Box<dyn Expr>>, Vec<Box<dyn Expr>>)>,
}

impl SwitchExpr {
    pub fn parse(parser: &mut Parser) -> Self {
        if parser.tokens()[parser.index()] != Token::Switch {
            panic!("expected 'switch'");
        }
        parser.bump();

        if parser.tokens()[parser.index()] != Token::LParen {
            panic!("expected '(' after switch");
        }
        parser.bump();
        let value = parser.set_can_multi(true, |parser| Box::new(parser.parse_expression()));
        if parser.tokens()[parser.index()] != Token::RParen {
            panic!("expected ')' after switch condition");
        }
        parser.bump();

        if parser.tokens()[parser.index()] != Token::LBrace {
            panic!("expected '{{' after switch condition");
        }
        parser.bump();

        let mut cases: Vec<(Option<Box<dyn Expr>>, Vec<Box<dyn Expr>>)> = Vec::new();
        let mut current_case: Option<usize> = None;

        while !matches!(parser.tokens()[parser.index()], Token::RBrace | Token::Eof) {
            match parser.tokens()[parser.index()] {
                Token::Case => {
                    parser.bump();
                    let cond = parser
                        .set_can_multi(false, |parser| expr::Operator::parse(parser));
                    if parser.tokens()[parser.index()] != Token::Colon {
                        panic!("expected ':' after case condition");
                    }
                    parser.bump();
                    cases.push((Some(cond), Vec::new()));
                    current_case = Some(cases.len() - 1);
                }
                Token::Default => {
                    parser.bump();
                    if parser.tokens()[parser.index()] != Token::Colon {
                        panic!("expected ':' after default");
                    }
                    parser.bump();
                    cases.push((None, Vec::new()));
                    current_case = Some(cases.len() - 1);
                }
                _ => {
                    if current_case.is_none() {
                        cases.push((None, Vec::new()));
                        current_case = Some(0);
                    }

                    let before = parser.index();
                    let mut exprs = parser.parse_expression();
                    if exprs.is_empty() && parser.index() == before {
                        parser.bump();
                        continue;
                    }
                    cases[current_case.expect("case index set")]
                        .1
                        .append(&mut exprs);
                }
            }
        }

        if parser.tokens()[parser.index()] == Token::RBrace {
            parser.bump();
        }

        Self { value, cases }
    }
}

impl Expr for SwitchExpr {
    fn compile(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        let value = self.value.compile(mem.clone());
        let cases: Vec<(Option<Vec<Code>>, Vec<Code>)> = self
            .cases
            .iter()
            .map(|(cond, body)| {
                (
                    cond.as_ref().map(|c| c.compile(mem.clone())),
                    body.compile(mem.clone()),
                )
            })
            .collect();

        vec![Box::new(move |proto, _| {
            logln(LogLevel::Trace, "Entering Expr::SwitchExpr");
            let target = handle_return!(run_sub(&value, proto.clone(), &mut CodeIndex::new()));
            let mut matched = false;

            for (cond, body) in &cases {
                if !matched {
                    matched = match cond {
                        Some(cond) => {
                            let v =
                                handle_return!(run_sub(cond, proto.clone(), &mut CodeIndex::new()));
                            {
                                let target_ref = target.borrow();
                                inline_borrow!(v).eq(&target_ref)
                            }
                        }
                        None => true,
                    };
                }

                if !matched {
                    continue;
                }

                match run_sub(body, proto.clone(), &mut CodeIndex::new()) {
                    CodeResult::Normal(_) | CodeResult::NormalMember(_, _, _) => {}
                    CodeResult::Break(None) => {
                        return CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)));
                    }
                    other => return other,
                }
            }

            CodeResult::Normal(Rc::new(RefCell::new(JsValue::Undefined)))
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            value: self.value.duplicate(),
            cases: self
                .cases
                .iter()
                .map(|(cond, body)| {
                    (
                        cond.as_ref().map(|a| a.duplicate()),
                        body.iter().map(|a| a.duplicate()).collect(),
                    )
                })
                .collect(),
        })
    }
}
