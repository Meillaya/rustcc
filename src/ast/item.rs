//! Top-level translation-unit items for the Rust-written C compiler.
//!
//! `Program` is intentionally small for chapters 1-8: it models a single
//! function definition (mirroring `nqcc2/lib/ast.ml`'s `Program [FunDecl ...]`
//! in its chapter-1 subset). Chapter 9 will widen `function` to
//! `Vec<Declaration>` to support multi-function translation units; the
//! chapter-1 parser/semantic owners stay unchanged.

use super::decl::Function;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Program {
    pub(crate) function: Function,
}
