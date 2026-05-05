//! Shared support utilities.
//!
//! Support modules hold cross-phase helpers that are not themselves AST, lexing,
//! parsing, semantic analysis, IR, or code generation.

pub(crate) mod diagnostics;
pub(crate) mod error;
pub(crate) mod source;
pub(crate) mod span;
