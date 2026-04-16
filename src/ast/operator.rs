//! Operator placeholders shared across expressions and lowering.

#![allow(dead_code)]

/// Placeholder unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    BitwiseNot,
    LogicalNot,
    AddressOf,
    Dereference,
    PreIncrement,
    PreDecrement,
    PostIncrement,
    PostDecrement,
}

/// Placeholder binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    LogicalAnd,
    LogicalOr,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
