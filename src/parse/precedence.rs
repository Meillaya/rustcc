//! Expression-precedence helpers.
//!
//! This file is reserved for precedence-climbing or another explicit
//! expression strategy once binary and logical operators arrive.

#![allow(dead_code)]

/// Placeholder precedence levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,
    Assignment,
    Conditional,
    LogicalOr,
    LogicalAnd,
    Equality,
    Relational,
    Additive,
    Multiplicative,
    Unary,
    Postfix,
    Primary,
}
