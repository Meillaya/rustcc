//! Optimization-pass scaffolding.
//!
//! Mirrors `nqcc2/lib/optimizations/optimize.ml`.  Each `OptPass` corresponds
//! to one chapter in the optimization arc of the book:
//! - `ConstantFolding` — chapter 20
//! - `UnreachableCodeElim` — chapter 20
//! - `CopyPropagation` — chapter 20
//! - `DeadStoreElim` — chapter 20
//!
//! The real implementations live in their own modules under
//! `nqcc2/lib/optimizations/` (e.g. `constant_folding.ml`, `copy_prop.ml`).
//! Until wave 20 wires each pass, `run_opt` is a stub that returns the
//! program unchanged so the pipeline continues to compile.

#![allow(dead_code)]

use crate::ir::tacky::TackyProgram;

/// Book-faithful optimization pass selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OptPass {
    ConstantFolding,
    UnreachableCodeElim,
    CopyPropagation,
    DeadStoreElim,
}

/// Run the selected optimization passes in order.
///
/// The real implementation constructs a CFG per function, runs each pass in
/// sequence with a fixed-point loop until no pass reports a change, and
/// reassembles the optimized TACKY program.  The placeholder exists so the
/// pipeline compiles before the wave-20 passes arrive.
pub fn run_opt(_program: TackyProgram, _passes: &[OptPass]) -> TackyProgram {
    unimplemented!()
}
