//! CFG-backed TACKY unreachable-code elimination pass.
//!
//! Mirrors nqcc2/lib/optimizations/unreachable_code_elim.ml:1-91 and is wired
//! from `opt.rs` in the same slot as nqcc2/lib/optimizations/optimize.ml:13-16.

use std::collections::BTreeSet;

use crate::ir::cfg::{self, BlockId, Cfg, NodeId};
use crate::ir::constant_folding::PassResult;
use crate::ir::tacky::{Instruction, TackyFunction, TackyProgram};

// Mirrors nqcc2/lib/optimizations/optimize.ml:37-46.
pub(crate) fn eliminate_unreachable_code_program(program: TackyProgram) -> PassResult {
    let mut changed = false;
    let functions = program
        .functions
        .into_iter()
        .map(|function| {
            let result = eliminate_unreachable_code_function(function);
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

fn eliminate_unreachable_code_function(mut function: TackyFunction) -> FunctionResult {
    let original_body = function.body.clone();
    let Ok(function_cfg) = cfg::tacky_function_cfg(&function) else {
        return FunctionResult {
            function,
            changed: false,
        };
    };

    let optimized_cfg = optimize(function_cfg.cfg);
    let body = optimized_cfg.cfg_to_instructions();
    let changed = body != original_body;

    // Preserve all original `TackyFunction` metadata while replacing only the
    // optimized body reassembled from the TACKY CFG.
    function.body = body;
    FunctionResult { function, changed }
}

// Mirrors nqcc2/lib/optimizations/unreachable_code_elim.ml:83-91.
fn optimize(cfg: Cfg<(), Instruction>) -> Cfg<(), Instruction> {
    let cfg = eliminate_unreachable_blocks(cfg);
    let cfg = eliminate_useless_jumps(cfg);
    let cfg = eliminate_useless_labels(cfg);
    remove_empty_blocks(cfg)
}

// Mirrors nqcc2/lib/optimizations/unreachable_code_elim.ml:13-36.
fn eliminate_unreachable_blocks(mut cfg: Cfg<(), Instruction>) -> Cfg<(), Instruction> {
    let reachable_block_ids = cfg.reachable_block_ids();
    let unreachable_block_ids = cfg
        .block_ids()
        .filter(|id| !reachable_block_ids.contains(id))
        .collect::<BTreeSet<_>>();

    for id in &unreachable_block_ids {
        let node = NodeId::Block(*id);
        let Some(block) = cfg.block(*id).cloned() else {
            continue;
        };
        for pred in block.preds {
            cfg.remove_edge(pred, node);
        }
        for succ in block.succs {
            cfg.remove_edge(node, succ);
        }
    }

    cfg.basic_blocks
        .retain(|block| reachable_block_ids.contains(&block.id));
    cfg
}

// Mirrors nqcc2/lib/optimizations/unreachable_code_elim.ml:38-54.
fn eliminate_useless_jumps(mut cfg: Cfg<(), Instruction>) -> Cfg<(), Instruction> {
    let block_count = cfg.basic_blocks.len();
    if block_count <= 1 {
        return cfg;
    }

    let default_successors = cfg
        .basic_blocks
        .iter()
        .skip(1)
        .map(|block| NodeId::Block(block.id))
        .collect::<Vec<_>>();

    for (idx, default_succ) in default_successors.into_iter().enumerate() {
        let block = &mut cfg.basic_blocks[idx];
        if is_jump(block.instructions.last().map(|(_, instr)| instr))
            && block.succs.iter().all(|succ| *succ == default_succ)
        {
            block.instructions.pop();
        }
    }
    cfg
}

fn is_jump(instruction: Option<&Instruction>) -> bool {
    matches!(
        instruction,
        Some(
            Instruction::Jump { .. }
                | Instruction::JumpIfZero { .. }
                | Instruction::JumpIfNotZero { .. }
        )
    )
}

// Mirrors nqcc2/lib/optimizations/unreachable_code_elim.ml:56-68.
fn eliminate_useless_labels(mut cfg: Cfg<(), Instruction>) -> Cfg<(), Instruction> {
    let default_predecessors = cfg
        .basic_blocks
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            if idx == 0 {
                NodeId::Entry
            } else {
                NodeId::Block(cfg.basic_blocks[idx - 1].id)
            }
        })
        .collect::<Vec<_>>();

    for (block, default_pred) in cfg.basic_blocks.iter_mut().zip(default_predecessors) {
        let starts_with_label = matches!(
            block.instructions.first().map(|(_, instr)| instr),
            Some(Instruction::Label(_))
        );
        if starts_with_label && block.preds.iter().all(|pred| *pred == default_pred) {
            block.instructions.remove(0);
        }
    }
    cfg
}

// Mirrors nqcc2/lib/optimizations/unreachable_code_elim.ml:70-81.
fn remove_empty_blocks(mut cfg: Cfg<(), Instruction>) -> Cfg<(), Instruction> {
    let empty_blocks = cfg
        .basic_blocks
        .iter()
        .filter(|block| block.instructions.is_empty())
        .map(|block| (block.id, block.preds.clone(), block.succs.clone()))
        .collect::<Vec<_>>();

    for (id, preds, succs) in &empty_blocks {
        if let ([pred], [succ]) = (preds.as_slice(), succs.as_slice()) {
            cfg.remove_edge(*pred, NodeId::Block(*id));
            cfg.remove_edge(NodeId::Block(*id), *succ);
            cfg.add_edge(*pred, *succ);
        }
    }

    let empty_ids = empty_blocks
        .into_iter()
        .map(|(id, _, _)| id)
        .collect::<BTreeSet<BlockId>>();
    cfg.basic_blocks
        .retain(|block| !empty_ids.contains(&block.id));
    cfg
}
