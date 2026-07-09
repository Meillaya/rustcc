//! Optimization-pass scaffolding.
//!
//! Mirrors `nqcc2/lib/optimizations/optimize.ml`.  Each `OptPass` corresponds
//! to one chapter in the optimization arc of the book:
//! - `ConstantFolding` — chapter 19
//! - `UnreachableCodeElim` — chapter 19
//! - `CopyPropagation` — chapter 19
//! - `DeadStoreElim` — chapter 19
//!
//! The real implementations live in their own modules under
//! `nqcc2/lib/optimizations/` (e.g. `constant_folding.ml`, `copy_prop.ml`).

#![allow(dead_code)]

use crate::ir::constant_folding::constant_fold_program;
use crate::ir::copy_propagation::propagate_copies_program;
use crate::ir::dead_store_elim::eliminate_dead_stores_program;
use crate::ir::tacky::TackyProgram;
use crate::ir::unreachable_code_elim::eliminate_unreachable_code_program;

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
/// Mirrors nqcc2/lib/optimizations/optimize.ml:4-35.  The full OCaml pipeline
/// reaches a fixed point over all enabled passes; this wave wires constant
/// folding, unreachable-code elimination, copy propagation, and dead-store
/// elimination in the book order until they reach a fixed point.
pub fn run_opt(program: TackyProgram, passes: &[OptPass]) -> TackyProgram {
    let mut current = program;
    loop {
        let mut changed = false;
        for pass in passes {
            current = match pass {
                OptPass::ConstantFolding => {
                    let result = constant_fold_program(current);
                    changed |= result.changed;
                    result.program
                }
                OptPass::UnreachableCodeElim => {
                    let result = eliminate_unreachable_code_program(current);
                    changed |= result.changed;
                    result.program
                }
                OptPass::CopyPropagation => {
                    let result = propagate_copies_program(current);
                    changed |= result.changed;
                    result.program
                }
                OptPass::DeadStoreElim => {
                    let result = eliminate_dead_stores_program(current);
                    changed |= result.changed;
                    result.program
                }
            };
        }
        if !changed {
            return current;
        }
    }
}

// Mirrors nqcc2/lib/optimizations/optimize.ml:37-46.
