//! Top-level translation-unit items for the Rust-written C compiler.
//!
//! Mirrors `nqcc2/lib/ast.ml`'s chapter-10 `Program [FunDecl | VarDecl ...]`
//! shape.  A translation unit is a sequence of top-level items, where
//! each item is a function definition (chapter 9), a function
//! declaration without a body (`int foo(int x);` — chapter 9), or a
//! file-scope variable declaration like `int g = 5;` or `static int h;`
//! (chapter 10).  Chapter 13 will add structs.

use super::decl::{Function, GlobalDecl, GlobalVarDecl, StructDecl};

/// A single top-level item in the translation unit.
///
/// Chapter 9 mirrors OCaml's `FunDecl { body = Some ... | None; ... }`
/// by splitting the body into `Definition` (some body) and `Declaration`
/// (no body).  Chapter 10 widens the surface with `Variable`, a
/// file-scope variable declaration carrying the type, optional
/// initializer, and storage-class specifier.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TopLevelItem {
    Function(Function),
    Declaration(GlobalDecl),
    Variable(GlobalVarDecl),
    StructDecl(StructDecl),
}

/// The complete AST for a translation unit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Program {
    pub(crate) top_level_items: Vec<TopLevelItem>,
}
