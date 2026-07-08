//! Declaration-like AST nodes.
//!
//! Rust structs are used for declaration records because the fields are fixed,
//! while enums are used where the grammar offers a closed set of alternatives.

use super::{expr::Expr, stmt::Statement, ty::Type};

/// A function parameter declaration.  Mirrors `function_declaration`'s
/// `params : string list` for chapter 9; we carry the parameter name and
/// its declaration shape so the resolve pass can scope parameter names
/// inside the function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VarDecl {
    pub(crate) name: String,
    pub(crate) init: Option<Expr>,
    /// Chapter-10 storage-class specifier.  `Auto` for plain
    /// `int x = 0;`, `Static` for `static int x;` (file-scope tentative
    /// or block-scope local), and `Extern` for `extern int x;`
    /// references inside a block.
    pub(crate) storage: StorageClass,
}

/// A function definition.  Mirrors `function_declaration { body = Some ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) params: Vec<VarDecl>,
    pub(crate) body: Option<Vec<BlockItem>>,
    /// Chapter-10 storage-class specifier on a function definition.
    /// `Auto` (the default for plain `int foo(void) { ... }`) carries
    /// external linkage; `Static` carries internal linkage.  Block
    /// scopes never see a function definition (C forbids nested
    /// functions) so this field is only meaningful at file scope.
    pub(crate) storage: StorageClass,
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
    /// Chapter-10 storage-class specifier on a function declaration.
    /// `Static` is rejected at resolve time for local declarations
    /// (`static int f(void);` inside a block); `Extern` and `Auto` are
    /// treated identically.
    pub(crate) storage: StorageClass,
}

/// Storage-class specifier attached to a file-scope variable declaration.
///
/// Chapter 10 only needs to distinguish two outcomes:
///
/// - `Static`   — internal linkage, no `.globl` directive emitted.
/// - `Extern`   — external linkage (default for file-scope vars too),
///                emits `.globl`.
/// - `Auto`     — placeholder meaning *no storage class keyword* (the
///                default for plain `int g = 5;`); behaves like `Extern`
///                for linkage purposes.
///
/// Mirrors `nqcc2/lib/ast.ml` `storage_class = Static | Extern`, extended
/// with an `Auto` arm so the Rust port can carry the "no keyword" case
/// explicitly (the OCaml reference uses `option<storage_class>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StorageClass {
    Static,
    Extern,
    Auto,
}

/// A file-scope variable declaration, e.g. `int g = 5;` or `static int g;`.
///
/// Mirrors `nqcc2/lib/ast.ml` `variable_declaration` — the OCaml shape
/// carries the type, optional initializer, and optional storage class.
/// Chapter 10 keeps `ty` to a single variant (`Type::Int`) but the field
/// is present so later chapters can drop in `long` / `unsigned` without
/// disturbing call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GlobalVarDecl {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) init: Option<Expr>,
    pub(crate) storage: StorageClass,
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
