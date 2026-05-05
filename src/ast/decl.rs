//! Declaration-like AST nodes.
//!
//! Rust structs are used for declaration records because the fields are fixed,
//! while enums are used where the grammar offers a closed set of alternatives.

use super::{expr::Expr, stmt::Statement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockItem {
    Declaration { name: String, init: Option<Expr> },
    Statement(Statement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ForInit {
    Declaration { name: String, init: Option<Expr> },
    Expr(Expr),
}
