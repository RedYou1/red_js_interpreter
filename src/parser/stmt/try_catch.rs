use std::{cell::RefCell, rc::Rc};

use crate::{CodeIndex, CodeResult, JsValue, Prototype, handle_error, parser::stmt::Stmt, run_sub};

#[derive(Debug)]
pub struct Try {
    pub block: Vec<Box<dyn Stmt>>,
    pub catch: Option<(Option<Vec<String>>, Vec<Box<dyn Stmt>>)>,
    pub finally: Option<Vec<Box<dyn Stmt>>>,
}

impl Stmt for Try {
    fn compile_stmt(
        &self,
        mem: std::rc::Rc<std::cell::RefCell<crate::Prototype>>,
    ) -> Vec<crate::Code> {
        let block = self.block.compile_stmt(mem.clone());
        let catch = self.catch.as_ref().map(|m| {
            (
                m.0.as_ref().map(|l| l.first().unwrap().clone()),
                m.1.compile_stmt(mem.clone()),
            )
        });
        let finally = self.finally.as_ref().map(|m| m.compile_stmt(mem.clone()));
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

    fn duplicate_stmt(&self) -> Box<dyn Stmt> {
        Box::new(Self {
            block: self.block.iter().map(|f| f.duplicate_stmt()).collect(),
            catch: self.catch.as_ref().map(|f| {
                (
                    f.0.clone(),
                    f.1.iter().map(|f| f.duplicate_stmt()).collect(),
                )
            }),
            finally: self
                .finally
                .as_ref()
                .map(|f| f.iter().map(|f| f.duplicate_stmt()).collect()),
        })
    }
}
