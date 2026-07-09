//! CFG-backed TACKY constant folding pass.
//!
//! Mirrors nqcc2/lib/optimizations/constant_folding.ml:173-175 and is wired
//! from `opt.rs` in the same slot as nqcc2/lib/optimizations/optimize.ml:9-12.

use std::collections::HashMap;

use crate::ir::cfg;
use crate::ir::const_eval::ConstVal;
use crate::ir::constant_folding::instr::optimize_instruction;
use crate::ir::tacky::{TackyFunction, TackyProgram};

mod folds;
mod instr;
mod util;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PassResult {
    pub(crate) program: TackyProgram,
    pub(crate) changed: bool,
}

// Mirrors nqcc2/lib/optimizations/optimize.ml:37-46.
pub(crate) fn constant_fold_program(program: TackyProgram) -> PassResult {
    let mut changed = false;
    let functions = program
        .functions
        .into_iter()
        .map(|function| {
            let result = constant_fold_function(function);
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

fn constant_fold_function(mut function: TackyFunction) -> FunctionResult {
    let Ok(mut function_cfg) = cfg::tacky_function_cfg(&function) else {
        return FunctionResult {
            function,
            changed: false,
        };
    };

    let mut changed = false;
    for block in function_cfg.cfg.blocks_mut() {
        let mut constants = HashMap::<String, ConstVal>::new();
        block.instructions = block
            .instructions
            .drain(..)
            .filter_map(|(annotation, instruction)| {
                let result = optimize_instruction(instruction, &function.type_env, &mut constants);
                changed |= result.changed;
                result
                    .instruction
                    .map(|instruction| (annotation, instruction))
            })
            .collect();
    }

    // Preserve all original `TackyFunction` metadata while replacing only the
    // optimized body reassembled from the TACKY CFG.
    function.body = function_cfg.cfg.cfg_to_instructions();
    FunctionResult { function, changed }
}
