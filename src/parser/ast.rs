use std::{cell::RefCell, rc::Rc};

use crate::{Prototype, Runnable};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    FunctionDecl(FunctionDecl),
    ClassDecl(ClassDecl),
    VarDecl(String, Option<Expr>),
    ExprStmt(Expr),
    Return(Option<Expr>),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    DoWhile(DoWhileStmt),
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub super_class: Option<Expr>,
    pub methods: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub consequent: Vec<Stmt>,
    pub alternate: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub condition: Option<Expr>,
    pub update: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoWhileStmt {
    pub body: Vec<Stmt>,
    pub condition: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier(String),
    Number(f64),
    String(String),
    TemplateLiteral(Vec<TemplatePart>),
    Boolean(bool),
    Object(Vec<(Expr, Expr)>),
    Array(Vec<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    New(Box<Expr>, Vec<Expr>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    PostfixDec(Box<Expr>),
    PostfixInc(Box<Expr>),
    FunctionExpr(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    String(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}

impl Program {
    pub fn compile(self, prebuild: Rc<RefCell<Prototype>>) -> Runnable {
        crate::parser::compiler::compile_program(prebuild.clone(), self)
    }
}
