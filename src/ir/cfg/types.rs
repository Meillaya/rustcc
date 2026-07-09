//! CFG data structures and reusable graph helpers.
//!
//! Mirrors `nqcc2/lib/cfg.ml:21-120` plus annotation helpers from
//! `nqcc2/lib/cfg.ml:167-188`.

use std::collections::{BTreeSet, VecDeque};

use crate::codegen::assembly;
use crate::ir::tacky;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeId {
    Entry,
    Block(BlockId),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct BasicBlock<V, I> {
    pub id: BlockId,
    pub instructions: Vec<(V, I)>,
    pub preds: Vec<NodeId>,
    pub succs: Vec<NodeId>,
    pub value: V,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cfg<V, I> {
    pub basic_blocks: Vec<BasicBlock<V, I>>,
    pub entry: NodeId,
    pub exit: NodeId,
    pub entry_succs: Vec<NodeId>,
    pub exit_preds: Vec<NodeId>,
    pub debug_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCfg<I> {
    pub name: String,
    pub cfg: Cfg<(), I>,
}

impl<V, I> Cfg<V, I> {
    pub fn get_succs(&self, node: NodeId) -> &[NodeId] {
        match node {
            NodeId::Entry => &self.entry_succs,
            NodeId::Block(id) => self.block(id).map_or(&[], |block| block.succs.as_slice()),
            NodeId::Exit => &[],
        }
    }

    pub fn get_block_value(&self, id: BlockId) -> Option<&V> {
        self.block(id).map(|block| &block.value)
    }

    pub fn block(&self, id: BlockId) -> Option<&BasicBlock<V, I>> {
        self.basic_blocks.iter().find(|block| block.id == id)
    }

    pub fn block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock<V, I>> {
        self.basic_blocks.iter_mut().find(|block| block.id == id)
    }

    pub fn block_ids(&self) -> impl Iterator<Item = BlockId> + '_ {
        self.basic_blocks.iter().map(|block| block.id)
    }

    pub fn blocks(&self) -> &[BasicBlock<V, I>] {
        &self.basic_blocks
    }

    pub fn blocks_mut(&mut self) -> &mut [BasicBlock<V, I>] {
        &mut self.basic_blocks
    }

    pub fn update_basic_block(&mut self, block: BasicBlock<V, I>) -> Option<BasicBlock<V, I>> {
        let slot = self
            .basic_blocks
            .iter_mut()
            .find(|existing| existing.id == block.id)?;
        Some(std::mem::replace(slot, block))
    }

    fn update_successors<F>(&mut self, node: NodeId, f: F)
    where
        F: FnOnce(&mut Vec<NodeId>),
    {
        match node {
            NodeId::Entry => f(&mut self.entry_succs),
            NodeId::Block(id) => {
                if let Some(block) = self.block_mut(id) {
                    f(&mut block.succs);
                }
            }
            NodeId::Exit => {}
        }
    }

    fn update_predecessors<F>(&mut self, node: NodeId, f: F)
    where
        F: FnOnce(&mut Vec<NodeId>),
    {
        match node {
            NodeId::Entry => {}
            NodeId::Block(id) => {
                if let Some(block) = self.block_mut(id) {
                    f(&mut block.preds);
                }
            }
            NodeId::Exit => f(&mut self.exit_preds),
        }
    }

    pub fn add_edge(&mut self, pred: NodeId, succ: NodeId) {
        self.update_successors(pred, |succs| push_unique(succs, succ));
        self.update_predecessors(succ, |preds| push_unique(preds, pred));
    }

    pub fn remove_edge(&mut self, pred: NodeId, succ: NodeId) {
        self.update_successors(pred, |succs| succs.retain(|id| *id != succ));
        self.update_predecessors(succ, |preds| preds.retain(|id| *id != pred));
    }

    pub fn reachable_block_ids(&self) -> BTreeSet<BlockId> {
        let mut seen_nodes = BTreeSet::new();
        let mut blocks = BTreeSet::new();
        let mut queue = VecDeque::from([NodeId::Entry]);
        while let Some(node) = queue.pop_front() {
            if !seen_nodes.insert(node) {
                continue;
            }
            if let NodeId::Block(id) = node {
                blocks.insert(id);
            }
            queue.extend(self.get_succs(node).iter().copied());
        }
        blocks
    }
}

impl<V: Clone, I: Clone> Cfg<V, I> {
    pub fn cfg_to_instructions(&self) -> Vec<I> {
        self.basic_blocks
            .iter()
            .flat_map(|block| block.instructions.iter().map(|(_, instr)| instr.clone()))
            .collect()
    }

    pub fn initialize_annotation<U: Clone>(&self, value: U) -> Cfg<U, I> {
        Cfg {
            basic_blocks: self
                .basic_blocks
                .iter()
                .map(|block| BasicBlock {
                    id: block.id,
                    instructions: block
                        .instructions
                        .iter()
                        .map(|(_, instr)| (value.clone(), instr.clone()))
                        .collect(),
                    preds: block.preds.clone(),
                    succs: block.succs.clone(),
                    value: value.clone(),
                })
                .collect(),
            entry: self.entry,
            exit: self.exit,
            entry_succs: self.entry_succs.clone(),
            exit_preds: self.exit_preds.clone(),
            debug_label: self.debug_label.clone(),
        }
    }

    pub fn strip_annotations(&self) -> Cfg<(), I> {
        self.initialize_annotation(())
    }
}

fn push_unique(ids: &mut Vec<NodeId>, id: NodeId) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

pub type TackyCfg<V = ()> = Cfg<V, tacky::Instruction>;
pub type AssemblyCfg<V = ()> = Cfg<V, assembly::Instr>;
