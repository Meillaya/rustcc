//! Native intermediate representation.
//!
//! The IR is consumed by the codegen and optimization passes.  The book-faithful
//! surface mirrors `nqcc2/lib/tacky.ml` (99 LOC) plus the surrounding scaffolding
//! that the Rust port needs for ergonomics:
//!
//! - `tacky` — TACKY program / function / instruction definitions and the
//!   `ast_to_tacky` lowering entry point.
//! - `lower` — AST-to-TACKY lowering helper.  Wave 0 ships a placeholder that
//!   the chapter-1 implementation will replace in W2-T2.
//! - `opt` — optimization-pass selector and runner.
//! - `cfg` — control-flow graph functor scaffold.
//! - `temp` — typed temporary-identifier generator used by lowering and regalloc.

// No runtime interpreter; the IR is consumed only by codegen and optimization.
//
// Re-exports below are part of the public IR surface and will be pulled in by
// codegen and optimization passes during waves 9-20.  The compiler pipeline
// currently imports types directly from each sub-module, so allow the unused
// public-use until downstream callers land.
#![allow(unused_imports)]

pub mod cfg;
pub mod const_eval;
mod constant_folding;
mod copy_propagation;
mod dead_store_elim;
pub mod lower;
pub mod opt;
pub mod tacky;
pub mod temp;
mod unreachable_code_elim;

pub use opt::{OptPass, run_opt};
pub use tacky::{Instruction, TackyFunction, TackyProgram, Val, Var, ast_to_tacky};
pub use temp::{TempId, TempIdGenerator};
