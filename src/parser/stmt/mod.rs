use std::{cell::RefCell, rc::Rc};

use crate::{Code, Prototype};

pub trait Stmt {
    fn compile_stmt(&self, mem: Rc<RefCell<Prototype>>) -> Vec<Code>;
}

mod if_decl;
mod loop_decl;

pub use {if_decl::IfStmt, loop_decl::LoopStmt};
