//! Statement node placeholders.

#![allow(dead_code)]

/// Placeholder statement node.
#[derive(Debug, Clone)]
pub enum Stmt {
    Return,
    Expression,
    Declaration,
    Block,
    If,
    While,
    DoWhile,
    For,
    Break,
    Continue,
    Switch,
    Label,
    Goto,
}
