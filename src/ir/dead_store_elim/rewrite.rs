use crate::ir::cfg::{BasicBlock, Cfg};
use crate::ir::tacky::{Instruction, Val};

use super::liveness::{LiveBlock, find_live_variables};
use super::util::{instruction_dst, pointer_base, update_address_facts};
use super::{AddressFacts, LiveSet};

// Mirrors nqcc2/lib/optimizations/dead_store_elim.ml:93-107.
pub(super) fn optimize(
    cfg: Cfg<(), Instruction>,
    static_vars: &LiveSet,
    aliased_vars: &LiveSet,
) -> Cfg<(), Instruction> {
    let annotated_cfg = find_live_variables(cfg, static_vars, aliased_vars);
    let basic_blocks = annotated_cfg
        .basic_blocks
        .iter()
        .map(rewrite_block)
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

fn rewrite_block(block: &LiveBlock) -> BasicBlock<(), Instruction> {
    let mut address_of = AddressFacts::new();
    let instructions = block
        .instructions
        .iter()
        .filter_map(|(live_vars, instruction)| {
            let dead_store = is_dead_store(live_vars, instruction)
                || is_dead_known_memory_store(live_vars, instruction, &address_of);
            update_address_facts(&mut address_of, instruction);
            (!dead_store).then(|| ((), instruction.clone()))
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

// Mirrors nqcc2/lib/optimizations/dead_store_elim.ml:84-91.
fn is_dead_store(live_vars: &LiveSet, instruction: &Instruction) -> bool {
    match instruction {
        Instruction::Call { .. } | Instruction::Store { .. } | Instruction::CopyBytes { .. } => {
            false
        }
        _ => instruction_dst(instruction).is_some_and(|dst| !live_vars.contains(&dst)),
    }
}

fn is_dead_known_memory_store(
    live_vars: &LiveSet,
    instruction: &Instruction,
    address_of: &AddressFacts,
) -> bool {
    let dst_pointer = match instruction {
        Instruction::Store { dst_pointer, .. } | Instruction::CopyBytes { dst_pointer, .. } => {
            dst_pointer
        }
        _ => return false,
    };
    let Some(base) = pointer_base(dst_pointer, address_of) else {
        return false;
    };
    !live_vars.contains(base)
}

pub(super) fn collapse_return_copies(instructions: Vec<Instruction>) -> Vec<Instruction> {
    let mut optimized = Vec::with_capacity(instructions.len());
    for instruction in instructions {
        if let Instruction::Return(Val::Var(name)) = &instruction
            && let Some(Instruction::Copy { src, dst }) = optimized.last()
            && dst == name
            && matches!(src, Val::Constant(_) | Val::ConstantDouble(_))
        {
            optimized.push(Instruction::Return(src.clone()));
            continue;
        }
        optimized.push(instruction);
    }
    optimized
}
