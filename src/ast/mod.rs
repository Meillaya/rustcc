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

pub(crate) use decl::{
    BlockItem, ForInit, Function, GlobalDecl, GlobalVarDecl, MemberDecl, StorageClass, StructDecl,
    VarDecl,
};
pub(crate) use expr::Expr;
pub(crate) use item::{Program, TopLevelItem};
pub(crate) use operator::{AssignOp, BinaryOp, UnaryOp};
pub(crate) use stmt::Statement;
pub(crate) use ty::Type;
