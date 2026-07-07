//! Shared support utilities.
//!
//! Support modules hold cross-phase helpers that are not themselves AST, lexing,
//! parsing, semantic analysis, IR, or code generation.

pub(crate) mod diagnostics;
pub(crate) mod error;
// source heuristics removed; the IR is consumed only by codegen.
pub(crate) mod span;
