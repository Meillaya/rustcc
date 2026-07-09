use std::collections::{BTreeMap, BTreeSet};

use crate::ir::cfg::{BasicBlock, Cfg};
use crate::ir::tacky::{Instruction, TypeEnv};

use super::dataflow::find_reaching_copies;
use super::facts::{CopyFact, ReachingCopies, ValKey, aggregate_copy_fact, update_address_facts};
use super::rewrite_support::{collect_write_pointers, replace_instruction_sources};

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:209-227.
pub(super) fn optimize(
    cfg: Cfg<(), Instruction>,
    type_env: &TypeEnv,
    static_vars: &std::collections::BTreeSet<String>,
    aliased_vars: &std::collections::BTreeSet<String>,
) -> Cfg<(), Instruction> {
    let annotated_cfg = find_reaching_copies(cfg, type_env, static_vars, aliased_vars);
    let write_pointers = collect_write_pointers(&annotated_cfg.basic_blocks);
    let basic_blocks = annotated_cfg
        .basic_blocks
        .iter()
        .map(|block| rewrite_block(block, type_env, &write_pointers))
        .collect();
    Cfg {
        basic_blocks,
        entry: annotated_cfg.entry,
        exit: annotated_cfg.exit,
        entry_succs: annotated_cfg.entry_succs,
        exit_preds: annotated_cfg.exit_preds,
        debug_label: annotated_cfg.debug_label,
    }
}

fn rewrite_block(
    block: &BasicBlock<ReachingCopies, Instruction>,
    type_env: &TypeEnv,
    write_pointers: &BTreeSet<String>,
) -> BasicBlock<(), Instruction> {
    let mut address_of = BTreeMap::<String, String>::new();
    let instructions = block
        .instructions
        .iter()
        .filter_map(|(copies, instruction)| {
            let rewritten =
                rewrite_instruction(copies, instruction, &address_of, type_env, write_pointers);
            update_address_facts(&mut address_of, instruction);
            rewritten
        })
        .collect();
    BasicBlock {
        id: block.id,
        instructions,
        preds: block.preds.clone(),
        succs: block.succs.clone(),
        value: (),
    }
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:153-207.
fn rewrite_instruction(
    copies: &ReachingCopies,
    instruction: &Instruction,
    address_of: &BTreeMap<String, String>,
    type_env: &TypeEnv,
    write_pointers: &BTreeSet<String>,
) -> Option<((), Instruction)> {
    if redundant_aggregate_copy(copies, instruction, address_of, type_env) {
        return None;
    }
    if redundant_scalar_copy(copies, instruction) {
        return None;
    }
    Some((
        (),
        replace_instruction_sources(instruction, copies, type_env, write_pointers),
    ))
}

fn redundant_aggregate_copy(
    copies: &ReachingCopies,
    instruction: &Instruction,
    address_of: &BTreeMap<String, String>,
    type_env: &TypeEnv,
) -> bool {
    let Instruction::CopyBytes {
        src_pointer,
        dst_pointer,
        ..
    } = instruction
    else {
        return false;
    };
    let Some(copy) = aggregate_copy_fact(src_pointer, dst_pointer, address_of, type_env) else {
        return false;
    };
    let reverse = CopyFact {
        src: copy.dst.clone(),
        dst: copy.src.clone(),
    };
    copies.contains(&copy) || copies.contains(&reverse)
}

fn redundant_scalar_copy(copies: &ReachingCopies, instruction: &Instruction) -> bool {
    let Instruction::Copy { src, dst } = instruction else {
        return false;
    };
    let copy = CopyFact {
        src: ValKey::from_val(src),
        dst: ValKey::Var(dst.clone()),
    };
    let reverse = CopyFact {
        src: ValKey::Var(dst.clone()),
        dst: ValKey::from_val(src),
    };
    copies.contains(&copy) || copies.contains(&reverse)
}
