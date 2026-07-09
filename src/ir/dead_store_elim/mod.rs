//! CFG-backed TACKY dead-store elimination pass.
//!
//! Mirrors nqcc2/lib/optimizations/dead_store_elim.ml:1-107 and is wired
//! from `opt.rs` in the same slot as nqcc2/lib/optimizations/optimize.ml:23-26.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::constant_folding::PassResult;
use crate::ir::tacky::{Instruction, TackyFunction, TackyProgram};

mod analysis;
mod liveness;
mod rewrite;
mod util;

use analysis::{analyze_address_taken, function_static_storage_vars};
use rewrite::optimize;

// Mirrors nqcc2/lib/optimizations/optimize.ml:37-46.
pub(crate) fn eliminate_dead_stores_program(program: TackyProgram) -> PassResult {
    let emitted_static_vars = program
        .static_variables
        .iter()
        .map(|var| var.name.clone())
        .collect::<BTreeSet<_>>();
    let mut changed = false;
    let functions = program
        .functions
        .into_iter()
        .map(|function| {
            let result = eliminate_dead_stores_function(function, &emitted_static_vars);
            changed |= result.changed;
            result.function
        })
        .collect();

    PassResult {
        program: TackyProgram {
            functions,
            static_variables: program.static_variables,
            static_constants: program.static_constants,
            function_param_types: program.function_param_types,
            function_return_types: program.function_return_types,
        },
        changed,
    }
}

struct FunctionResult {
    function: TackyFunction,
    changed: bool,
}

fn eliminate_dead_stores_function(
    mut function: TackyFunction,
    emitted_static_vars: &BTreeSet<String>,
) -> FunctionResult {
    let original_body = function.body.clone();
    let static_vars = function_static_storage_vars(&function, emitted_static_vars);
    let aliased_vars = analyze_address_taken(&function.body);
    let Ok(function_cfg) = crate::ir::cfg::tacky_function_cfg(&function) else {
        return FunctionResult {
            function,
            changed: false,
        };
    };

    let optimized_cfg = optimize(function_cfg.cfg, &static_vars, &aliased_vars);
    let body = rewrite::collapse_return_copies(optimized_cfg.cfg_to_instructions());
    let changed = body != original_body;

    // Preserve all original `TackyFunction` metadata while replacing only the
    // optimized body reassembled from the TACKY CFG.
    function.body = body;
    FunctionResult { function, changed }
}

pub(super) type LiveSet = BTreeSet<String>;
pub(super) type AddressFacts = BTreeMap<String, String>;
