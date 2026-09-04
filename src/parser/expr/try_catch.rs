use crate::{
    CodeIndex, CodeResult, Environment, LogLevel, Prototype,
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
        parser
            .env
            .logger
            .borrow_mut()
            .logln_str(LogLevel::Info, "Entering Try::parse");
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
        parser.env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
            format!(
                "Exiting Try::parse has_catch={} has_finally={}",
                catch.is_some(),
                finally.is_some()
            )
        });
        Self {
            block,
            catch,
            finally,
        }
    }
}

impl Expr for Try {
    fn compile(&self, env: Environment) -> Vec<crate::Code> {
        let block = self.block.compile(env.clone());
        let catch = self.catch.as_ref().map(|m| {
            (
                m.0.as_ref().map(|l| l.first().unwrap().clone()),
                m.1.compile(env.clone()),
            )
        });
        let finally = self.finally.as_ref().map(|m| m.compile(env.clone()));
        vec![Box::new(move |env, _| {
            env.logger
                .borrow_mut()
                .logln_str(LogLevel::Trace, "Entering Expr::Try block");
            let mut res = run_sub(
                block.as_ref(),
                env.with_mem(Prototype::new_child(env.mem.clone(), None, [])),
                &mut CodeIndex::new(),
            );
            if let Some((param, catch)) = catch.as_ref()
                && let CodeResult::Error(ref err) = res
            {
                env.logger.borrow_mut().logln(LogLevel::Trace, &|| {
                    format!("Entering Expr::Try catch after block error {err:?}")
                });
                let child = Prototype::new_child(
                    env.mem.clone(),
                    None,
                    if let Some(name) = param {
                        vec![(name.as_str().into(), err.clone())]
                    } else {
                        vec![]
                    },
                );
                let t = run_sub(catch.as_ref(), env.with_mem(child), &mut CodeIndex::new());
                res = t;
            }
            if let Some(finally) = finally.as_ref() {
                env.logger
                    .borrow_mut()
                    .logln_str(LogLevel::Trace, "Entering Expr::Try finally");
                let t = run_sub(
                    finally.as_ref(),
                    env.with_mem(Prototype::new_child(env.mem.clone(), None, [])),
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
