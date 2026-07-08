//! Top-level translation-unit items for the Rust-written C compiler.
//!
//! Mirrors `nqcc2/lib/ast.ml`'s chapter-9 `Program [FunDecl ...]` shape.
//! A translation unit is a sequence of top-level items, where each item is
//! currently a function definition (chapter 9) or a function declaration
//! without a body (`int foo(int x);` — also chapter 9).  Chapter 10 will
//! widen `TopLevelItem` with file-scope variable declarations; chapter 13
//! will add structs.  The current wave keeps `TopLevelItem` to the two
//! function variants so the parse / resolve / codegen pipeline can mirror
//! the OCaml reference's chapter-9 surface.

use super::decl::{Function, GlobalDecl};

/// A single top-level item in the translation unit.
///
/// Chapter 9 mirrors OCaml's `FunDecl { body = Some ... | None; ... }`
/// by splitting the body into `Definition` (some body) and `Declaration`
/// (no body).  Chapter 10 will add `GlobalVariable` here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TopLevelItem {
    Function(Function),
    Declaration(GlobalDecl),
}

/// The complete AST for a translation unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Program {
    pub(crate) top_level_items: Vec<TopLevelItem>,
}