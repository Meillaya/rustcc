//! Linear-instruction to CFG construction.
//!
//! Mirrors `nqcc2/lib/cfg.ml:124-164` and exposes TACKY/assembly convenience
//! builders for optimization and later liveness passes.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::codegen::assembly;
use crate::ir::cfg::instr::{CfgInstruction, SimpleInstr};
use crate::ir::cfg::types::{BasicBlock, BlockId, Cfg, FunctionCfg, NodeId};
use crate::ir::tacky::{self, TackyFunction, TackyProgram};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CfgBuildError {
    debug_label: String,
    target: String,
}

impl CfgBuildError {
    fn missing_label(debug_label: &str, target: &str) -> Self {
        Self {
            debug_label: debug_label.to_string(),
            target: target.to_string(),
        }
    }
}

impl fmt::Display for CfgBuildError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "CFG '{}' jumps to missing label '{}'",
            self.debug_label, self.target
        )
    }
}

impl Error for CfgBuildError {}

pub fn build<I>(debug_label: &str, instructions: &[I]) -> Result<Cfg<(), I>, CfgBuildError>
where
    I: CfgInstruction + Clone,
{
    let blocks = partition_into_basic_blocks(instructions);
    let mut cfg = Cfg {
        basic_blocks: blocks
            .into_iter()
            .enumerate()
            .map(|(idx, block)| BasicBlock {
                id: BlockId(idx),
                instructions: block.into_iter().map(|instr| ((), instr)).collect(),
                preds: Vec::new(),
                succs: Vec::new(),
                value: (),
            })
            .collect(),
        entry: NodeId::Entry,
        exit: NodeId::Exit,
        entry_succs: Vec::new(),
        exit_preds: Vec::new(),
        debug_label: debug_label.to_string(),
    };
    add_all_edges(&mut cfg)?;
    Ok(cfg)
}

pub fn build_tacky_program(
    program: &TackyProgram,
) -> Result<Vec<FunctionCfg<tacky::Instruction>>, CfgBuildError> {
    program.functions.iter().map(tacky_function_cfg).collect()
}

pub fn tacky_function_cfg(
    function: &TackyFunction,
) -> Result<FunctionCfg<tacky::Instruction>, CfgBuildError> {
    Ok(FunctionCfg {
        name: function.name.clone(),
        cfg: build(&function.name, &function.body)?,
    })
}

pub fn assembly_function_cfg(
    debug_label: &str,
    instructions: &[assembly::Instr],
) -> Result<Cfg<(), assembly::Instr>, CfgBuildError> {
    build(debug_label, instructions)
}

fn partition_into_basic_blocks<I>(instructions: &[I]) -> Vec<Vec<I>>
where
    I: CfgInstruction + Clone,
{
    let mut finished = Vec::new();
    let mut current = Vec::new();
    for instr in instructions {
        match instr.simplify() {
            SimpleInstr::Label(_) => {
                if !current.is_empty() {
                    finished.push(std::mem::take(&mut current));
                }
                current.push(instr.clone());
            }
            SimpleInstr::ConditionalJump(_)
            | SimpleInstr::UnconditionalJump(_)
            | SimpleInstr::Return => {
                current.push(instr.clone());
                finished.push(std::mem::take(&mut current));
            }
            SimpleInstr::Other => current.push(instr.clone()),
        }
    }
    if !current.is_empty() {
        finished.push(current);
    }
    finished
}

fn add_all_edges<I>(cfg: &mut Cfg<(), I>) -> Result<(), CfgBuildError>
where
    I: CfgInstruction,
{
    let label_map = cfg
        .basic_blocks
        .iter()
        .filter_map(|block| match block.instructions.first() {
            Some(((), instr)) => match instr.simplify() {
                SimpleInstr::Label(label) => Some((label.to_string(), NodeId::Block(block.id))),
                SimpleInstr::ConditionalJump(_)
                | SimpleInstr::UnconditionalJump(_)
                | SimpleInstr::Return
                | SimpleInstr::Other => None,
            },
            None => None,
        })
        .collect::<HashMap<_, _>>();

    if cfg.basic_blocks.is_empty() {
        cfg.add_edge(NodeId::Entry, NodeId::Exit);
        return Ok(());
    }

    cfg.add_edge(NodeId::Entry, NodeId::Block(BlockId(0)));
    let last_id = cfg.basic_blocks.last().map(|block| block.id);
    let edges = cfg
        .basic_blocks
        .iter()
        .filter_map(|block| {
            block
                .instructions
                .last()
                .map(|(_, instr)| (block.id, instr))
        })
        .map(|(id, instr)| block_edges(id, instr, last_id, &label_map, &cfg.debug_label))
        .collect::<Result<Vec<_>, _>>()?;

    for (pred, succs) in edges {
        for succ in succs {
            cfg.add_edge(pred, succ);
        }
    }
    Ok(())
}

fn block_edges<I>(
    id: BlockId,
    instr: &I,
    last_id: Option<BlockId>,
    label_map: &HashMap<String, NodeId>,
    debug_label: &str,
) -> Result<(NodeId, Vec<NodeId>), CfgBuildError>
where
    I: CfgInstruction,
{
    let pred = NodeId::Block(id);
    let next = if Some(id) == last_id {
        NodeId::Exit
    } else {
        NodeId::Block(BlockId(id.0 + 1))
    };
    let succs = match instr.simplify() {
        SimpleInstr::Return => vec![NodeId::Exit],
        SimpleInstr::UnconditionalJump(target) => {
            vec![label_target(label_map, debug_label, target)?]
        }
        SimpleInstr::ConditionalJump(target) => {
            vec![next, label_target(label_map, debug_label, target)?]
        }
        SimpleInstr::Label(_) | SimpleInstr::Other => vec![next],
    };
    Ok((pred, succs))
}

fn label_target(
    label_map: &HashMap<String, NodeId>,
    debug_label: &str,
    target: &str,
) -> Result<NodeId, CfgBuildError> {
    label_map
        .get(target)
        .copied()
        .ok_or_else(|| CfgBuildError::missing_label(debug_label, target))
}
