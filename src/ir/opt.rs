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
/// folding and unreachable-code elimination while leaving the later pass
/// selectors as no-ops for W20-T4..T5.
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
                OptPass::CopyPropagation | OptPass::DeadStoreElim => current,
            };
        }
        if !changed {
            return current;
        }
    }
}

// Mirrors nqcc2/lib/optimizations/optimize.ml:37-46.
