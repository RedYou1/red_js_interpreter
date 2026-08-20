use crate::{
    CodeIndex, CodeResult, Prototype,
    parser::{expr::Expr, lexer::Token, parser::Parser},
    run_sub,
};

#[derive(Debug)]
pub struct Try {
    pub block: Vec<Box<dyn Expr>>,
    pub catch: Option<(Option<Vec<String>>, Vec<Box<dyn Expr>>)>,
    pub finally: Option<Vec<Box<dyn Expr>>>,
}

impl Try {
    pub fn parse(parser: &mut Parser) -> Self {
        let block = parser.parse_block();
        let catch = if let Token::Catch = &parser.tokens()[parser.index()] {
            parser.bump();
            Some((
                if let Token::LParen = &parser.tokens()[parser.index()] {
                    Some(parser.parse_param_list())
                } else {
                    None
                },
                parser.parse_block(),
            ))
        } else {
            None
        };
        let finally = if let Token::Finally = &parser.tokens()[parser.index()] {
            parser.bump();
            Some(parser.parse_block())
        } else {
            None
        };
        Self {
            block,
            catch,
            finally,
        }
    }
}

impl Expr for Try {
    fn compile(&self, mem: std::rc::Rc<std::cell::RefCell<crate::Prototype>>) -> Vec<crate::Code> {
        let block = self.block.compile(mem.clone());
        let catch = self.catch.as_ref().map(|m| {
            (
                m.0.as_ref().map(|l| l.first().unwrap().clone()),
                m.1.compile(mem.clone()),
            )
        });
        let finally = self.finally.as_ref().map(|m| m.compile(mem.clone()));
        vec![Box::new(move |proto, _| {
            let mut res = run_sub(
                block.as_ref(),
                Prototype::new_child(proto.clone(), None, []),
                &mut CodeIndex::new(),
            );
            if let Some((param, catch)) = catch.as_ref()
                && let CodeResult::Error(ref err) = res
            {
                let child = Prototype::new_child(
                    proto.clone(),
                    None,
                    if let Some(name) = param {
                        vec![(name.as_str().into(), err.clone())]
                    } else {
                        vec![]
                    },
                );
                let t = run_sub(catch.as_ref(), child, &mut CodeIndex::new());
                if matches!(t, CodeResult::Return(_) | CodeResult::Error(_)) {
                    res = t;
                }
            }
            if let Some(finally) = finally.as_ref() {
                let t = run_sub(
                    finally.as_ref(),
                    Prototype::new_child(proto.clone(), None, []),
                    &mut CodeIndex::new(),
                );
                if matches!(t, CodeResult::Return(_) | CodeResult::Error(_)) {
                    res = t;
                }
            }
            res
        })]
    }

    fn duplicate(&self) -> Box<dyn Expr> {
        Box::new(Self {
            block: self.block.iter().map(|f| f.duplicate()).collect(),
            catch: self
                .catch
                .as_ref()
                .map(|f| (f.0.clone(), f.1.iter().map(|f| f.duplicate()).collect())),
            finally: self
                .finally
                .as_ref()
                .map(|f| f.iter().map(|f| f.duplicate()).collect()),
        })
    }
}
