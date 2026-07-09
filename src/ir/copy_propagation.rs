//! CFG-backed TACKY copy propagation pass.
//!
//! Mirrors nqcc2/lib/optimizations/copy_prop.ml:1-227 and is wired from
//! `opt.rs` in the same slot as nqcc2/lib/optimizations/optimize.ml:20-22.

use std::collections::BTreeSet;

use crate::ir::cfg;
use crate::ir::constant_folding::PassResult;
use crate::ir::tacky::{Instruction, TackyFunction, TackyProgram};

mod cleanup;
mod dataflow;
mod facts;
mod rewrite;

use cleanup::cleanup_unused_aggregate_scaffolding;
use rewrite::optimize;

// Mirrors nqcc2/lib/optimizations/optimize.ml:37-46.
pub(crate) fn propagate_copies_program(program: TackyProgram) -> PassResult {
    let static_vars = program
        .static_variables
        .iter()
        .map(|var| var.name.clone())
        .collect::<BTreeSet<_>>();
    let mut changed = false;
    let functions = program
        .functions
        .into_iter()
        .map(|function| {
            let result = propagate_copies_function(function, &static_vars);
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

fn propagate_copies_function(
    mut function: TackyFunction,
    static_vars: &BTreeSet<String>,
) -> FunctionResult {
    let original_body = function.body.clone();
    let aliased_vars = analyze_address_taken(&function.body);
    let Ok(function_cfg) = cfg::tacky_function_cfg(&function) else {
        return FunctionResult {
            function,
            changed: false,
        };
    };
    let optimized_cfg = optimize(
        function_cfg.cfg,
        &function.type_env,
        static_vars,
        &aliased_vars,
    );
    let body = cleanup_unused_aggregate_scaffolding(optimized_cfg.cfg_to_instructions());
    let changed = body != original_body;
    // Preserve all original `TackyFunction` metadata while replacing only the
    // optimized body reassembled from the TACKY CFG.
    function.body = body;
    FunctionResult { function, changed }
}

// Mirrors nqcc2/lib/optimizations/address_taken.ml:3-9.
fn analyze_address_taken(instructions: &[Instruction]) -> BTreeSet<String> {
    instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::GetAddress { src, .. } => Some(src.clone()),
            _ => None,
        })
        .collect()
}
