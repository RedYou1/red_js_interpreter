use std::{cell::RefCell, rc::Rc};

use crate::{Code, Prototype};

pub trait Stmt {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code>;
    fn duplicate_stmt(&self) -> Box<dyn Stmt>;
}

impl Stmt for Vec<Box<dyn Stmt>> {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code> {
        self.iter()
            .flat_map(|stmt| stmt.compile_stmt(mem.clone()))
            .collect()
    }

    fn duplicate_stmt(&self) -> Box<dyn Stmt> {
        Box::new(
            self.iter()
                .map(|a| a.as_ref().duplicate_stmt())
                .collect::<Self>(),
        )
    }
}

mod if_decl;
mod loop_decl;

pub use {if_decl::IfStmt, loop_decl::LoopStmt};
