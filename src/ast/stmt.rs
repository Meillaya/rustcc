//! Statement AST variants.
//!
//! `Box<Statement>` keeps recursive statement variants finite-sized while still
//! making ownership explicit: each parent owns its child statement subtree.

use super::{
    decl::{BlockItem, ForInit},
    expr::Expr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Statement {
    Return(Expr),
    Block(Vec<BlockItem>),
    While {
        condition: Expr,
        body: Box<Statement>,
    },
    DoWhile {
        body: Box<Statement>,
        condition: Expr,
    },
    For {
        init: Option<ForInit>,
        condition: Option<Expr>,
        post: Option<Expr>,
        body: Box<Statement>,
    },
    Break,
    Continue,
    Switch {
        expr: Expr,
        body: Box<Statement>,
    },
    Case {
        value: Expr,
        statement: Box<Statement>,
    },
    Default {
        statement: Box<Statement>,
    },
    If {
        condition: Expr,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    Goto(String),
    Labeled {
        label: String,
        statement: Box<Statement>,
    },
    Expr(Option<Expr>),
}
