//! Abstract syntax tree for the C subset currently handled natively.
//!
//! These modules are intentionally data-focused: parsing constructs the tree,
//! semantic analysis resolves names over it, and IR lowering consumes it.

pub mod decl;
pub mod expr;
pub mod item;
pub mod operator;
pub mod stmt;
pub mod ty;

pub(crate) use decl::{BlockItem, ForInit, Function};
pub(crate) use expr::Expr;
pub(crate) use item::Program;
pub(crate) use operator::{AssignOp, BinaryOp};
pub(crate) use stmt::Statement;
