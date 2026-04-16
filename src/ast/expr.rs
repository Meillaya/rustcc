//! Expression node placeholders.

#![allow(dead_code)]

/// Placeholder expression node.
#[derive(Debug, Clone)]
pub enum Expr {
    IntegerLiteral,
    FloatLiteral,
    StringLiteral,
    Identifier,
    Unary,
    Binary,
    Assignment,
    Conditional,
    Call,
    Member,
    Subscript,
}
