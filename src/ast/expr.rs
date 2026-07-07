//! Expression AST variants.
//!
//! Expressions form a recursive tree, so child expressions are boxed. `enum`
//! plus pattern matching makes every compiler phase handle each expression kind
//! explicitly.

use super::operator::{AssignOp, BinaryOp, UnaryOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expr {
    Constant(i32),
    Var(String),
    Paren(Box<Expr>),
    /// A unary operation. Carries the operator kind via [`UnaryOp`] so the
    /// parser, lowerer, and codegen can dispatch on a single field.  Covers
    /// `-` (`Negate`), `~` (`Complement`), and chapter-4 `!` (`Not`).
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    PreInc(Box<Expr>),
    PreDec(Box<Expr>),
    PostInc(Box<Expr>),
    PostDec(Box<Expr>),
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    /// A binary operation. Carries the operator kind via [`BinaryOp`] so the
    /// parser, lowerer, and codegen can dispatch on a single field.  Covers
    /// chapter-3 arithmetic / bitwise / shift operators and the chapter-4
    /// equality / relational / logical operators.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    /// Return the variable name when this expression can be assigned to.
    /// Parentheses preserve lvalue-ness for cases like `++(a)`, but assignment
    /// results do not become lvalues, matching the invalid increment tests.
    pub(crate) fn lvalue_name(&self) -> Option<&str> {
        match self {
            Self::Var(name) => Some(name),
            Self::Paren(inner) => inner.lvalue_name(),
            _ => None,
        }
    }
}