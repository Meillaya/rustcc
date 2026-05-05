//! Top-level translation-unit items for the Rust-written C compiler.
//!
//! `Program` is intentionally small for the current native early-chapter
//! interpreter: it models one function body. Later full-C work can widen this
//! module without changing parser/semantic ownership.

use super::decl::BlockItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Program {
    pub(crate) function_name: String,
    pub(crate) body: Vec<BlockItem>,
}
