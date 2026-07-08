//! Declaration-like AST nodes.
//!
//! Rust structs are used for declaration records because the fields are fixed,
//! while enums are used where the grammar offers a closed set of alternatives.

use super::{expr::Expr, stmt::Statement};

/// A function parameter declaration.  Mirrors `function_declaration`'s
/// `params : string list` for chapter 9; we carry the parameter name and
/// its declaration shape so the resolve pass can scope parameter names
/// inside the function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VarDecl {
    pub(crate) name: String,
    pub(crate) init: Option<Expr>,
}

/// A function definition.  Mirrors `function_declaration { body = Some ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) params: Vec<VarDecl>,
    pub(crate) body: Option<Vec<BlockItem>>,
}

/// A function declaration without a body.  Mirrors
/// `function_declaration { body = None }`, used for forward declarations
/// like `int foo(int x);`.  We keep `params` populated because chapter 9
/// treats the parameter names as informative (they may differ between
/// declarations and the definition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GlobalDecl {
    pub(crate) name: String,
    pub(crate) params: Vec<VarDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockItem {
    Declaration(VarDecl),
    Statement(Statement),
    /// A local function declaration without a body, e.g. `int foo(int x);`
    /// inside a block.  Reuses the `GlobalDecl` shape so the resolve pass
    /// can register the name (and arity) in the per-block scope; the
    /// lowerer treats this as a no-op.
    FunctionDecl(GlobalDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ForInit {
    Declaration(VarDecl),
    Expr(Expr),
}