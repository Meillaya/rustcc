use std::collections::VecDeque;

use crate::ir::cfg::{BasicBlock, BlockId, Cfg, NodeId};
use crate::ir::tacky::{Instruction, Val};

use super::util::{
    add_instruction_uses, add_known_memory_uses, instruction_dst, update_address_facts,
};
use super::{AddressFacts, LiveSet};

pub(super) type LiveCfg = Cfg<LiveSet, Instruction>;
pub(super) type LiveBlock = BasicBlock<LiveSet, Instruction>;

// Mirrors nqcc2/lib/backward_dataflow.ml:12-39.
pub(super) fn find_live_variables(
    cfg: Cfg<(), Instruction>,
    static_vars: &LiveSet,
    aliased_vars: &LiveSet,
) -> LiveCfg {
    let static_and_aliased_vars = static_vars.union(aliased_vars).cloned().collect();
    let mut current_cfg = cfg.initialize_annotation(LiveSet::new());
    let mut block_ids = current_cfg.block_ids().collect::<Vec<_>>();
    block_ids.reverse();
    let mut worklist = block_ids.into_iter().collect::<VecDeque<_>>();

    while let Some(block_id) = worklist.pop_front() {
        let Some(block) = current_cfg.block(block_id).cloned() else {
            continue;
        };
        let old_annotation = block.value.clone();
        let end_live_variables = meet(static_vars, &current_cfg, &block);
        let block = transfer(block, &static_and_aliased_vars, end_live_variables);
        let changed = old_annotation != block.value;
        let preds = block.preds.clone();
        current_cfg.update_basic_block(block);
        if changed {
            enqueue_predecessors(&mut worklist, preds);
        }
    }
    current_cfg
}

fn enqueue_predecessors(worklist: &mut VecDeque<BlockId>, preds: Vec<NodeId>) {
    for pred in preds {
        if let NodeId::Block(id) = pred
            && !worklist.contains(&id)
        {
            worklist.push_back(id);
        }
    }
}

// Mirrors nqcc2/lib/optimizations/dead_store_elim.ml:72-78.
fn meet(static_vars: &LiveSet, cfg: &LiveCfg, block: &LiveBlock) -> LiveSet {
    let mut live = LiveSet::new();
    for succ in &block.succs {
        match succ {
            NodeId::Entry => {}
            NodeId::Exit => live.extend(static_vars.iter().cloned()),
            NodeId::Block(id) => {
                if let Some(value) = cfg.get_block_value(*id) {
                    live.extend(value.iter().cloned());
                }
            }
        }
    }
    live
}

// Mirrors nqcc2/lib/optimizations/dead_store_elim.ml:6-70.
fn transfer(
    mut block: LiveBlock,
    static_and_aliased_vars: &LiveSet,
    end_live_variables: LiveSet,
) -> LiveBlock {
    let address_of = address_facts(&block);
    let mut current_live_vars = end_live_variables;
    let mut annotated_reversed = Vec::with_capacity(block.instructions.len());
    for (_, instruction) in block.instructions.into_iter().rev() {
        let annotation = current_live_vars.clone();
        current_live_vars = transfer_instruction(
            current_live_vars,
            &instruction,
            static_and_aliased_vars,
            &address_of,
        );
        annotated_reversed.push((annotation, instruction));
    }
    annotated_reversed.reverse();
    block.instructions = annotated_reversed;
    block.value = current_live_vars;
    block
}

fn address_facts(block: &LiveBlock) -> AddressFacts {
    let mut address_of = AddressFacts::new();
    for (_, instruction) in &block.instructions {
        update_address_facts(&mut address_of, instruction);
    }
    address_of
}

fn transfer_instruction(
    mut live_vars: LiveSet,
    instruction: &Instruction,
    static_and_aliased_vars: &LiveSet,
    address_of: &AddressFacts,
) -> LiveSet {
    if let Some(dst) = instruction_dst(instruction) {
        live_vars.remove(&dst);
    }
    add_instruction_uses(&mut live_vars, instruction);
    add_known_memory_uses(&mut live_vars, instruction, address_of);
    if matches!(
        instruction,
        Instruction::Call { .. } | Instruction::Load { .. }
    ) {
        live_vars.extend(static_and_aliased_vars.iter().cloned());
    }
    live_vars
}

pub(super) fn add_val(live_vars: &mut LiveSet, val: &Val) {
    if let Val::Var(name) = val {
        live_vars.insert(name.clone());
    }
}
