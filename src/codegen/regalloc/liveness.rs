// Mirrors nqcc2/lib/backend/regalloc.ml:227-263.

use std::collections::VecDeque;

use crate::codegen::assembly::Instr;
use crate::ir::cfg::{AssemblyCfg, BasicBlock, BlockId, Cfg, NodeId};

use super::operands::regs_used_and_written;
use super::types::{
    BlockLiveness, LiveMap, LiveSet, LivenessConfig, LivenessError, RegisterClass, regs_to_operands,
};

pub type LiveCfg = Cfg<LiveSet, Instr>;
pub type LiveBlock = BasicBlock<LiveSet, Instr>;

pub fn analyze(
    cfg: AssemblyCfg,
    class: RegisterClass,
    config: &LivenessConfig,
) -> Result<LiveCfg, LivenessError> {
    let mut current_cfg = cfg.initialize_annotation(LiveSet::new());
    let mut block_ids = current_cfg.block_ids().collect::<Vec<_>>();
    block_ids.reverse();
    let mut worklist = block_ids.into_iter().collect::<VecDeque<_>>();

    while let Some(block_id) = worklist.pop_front() {
        let Some(block) = current_cfg.block(block_id).cloned() else {
            continue;
        };
        let old_annotation = block.value.clone();
        let end_live_regs = meet(&current_cfg, &block, class, config);
        let block = transfer(block, class, config, end_live_regs)?;
        let changed = old_annotation != block.value;
        let preds = block.preds.clone();
        current_cfg.update_basic_block(block);
        if changed {
            enqueue_predecessors(&mut worklist, preds);
        }
    }
    Ok(current_cfg)
}

pub fn block_liveness(cfg: &LiveCfg) -> LiveMap {
    cfg.blocks()
        .iter()
        .map(|block| {
            let live_out = block
                .instructions
                .last()
                .map_or_else(LiveSet::new, |(live_after, _)| live_after.clone());
            (
                block.id,
                BlockLiveness {
                    live_in: block.value.clone(),
                    live_out,
                },
            )
        })
        .collect()
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

fn meet(
    cfg: &LiveCfg,
    block: &LiveBlock,
    class: RegisterClass,
    config: &LivenessConfig,
) -> LiveSet {
    let all_hardregs = regs_to_operands(&class.all_hardregs());
    let live_at_exit = regs_to_operands(&config.return_regs)
        .intersection(&all_hardregs)
        .cloned()
        .collect::<LiveSet>();
    let mut live = LiveSet::new();
    for succ in &block.succs {
        match succ {
            NodeId::Entry => {}
            NodeId::Exit => live.extend(live_at_exit.iter().cloned()),
            NodeId::Block(id) => {
                if let Some(value) = cfg.get_block_value(*id) {
                    live.extend(value.iter().cloned());
                }
            }
        }
    }
    live
}

fn transfer(
    mut block: LiveBlock,
    class: RegisterClass,
    config: &LivenessConfig,
    end_live_regs: LiveSet,
) -> Result<LiveBlock, LivenessError> {
    let mut current_live_regs = end_live_regs;
    let mut annotated_reversed = Vec::with_capacity(block.instructions.len());
    for (_, instruction) in block.instructions.into_iter().rev() {
        let annotation = current_live_regs.clone();
        let use_def = regs_used_and_written(&instruction, class, config)?;
        current_live_regs.retain(|reg| !use_def.written.contains(reg));
        current_live_regs.extend(use_def.used);
        annotated_reversed.push((annotation, instruction));
    }
    annotated_reversed.reverse();
    block.instructions = annotated_reversed;
    block.value = current_live_regs;
    Ok(block)
}
