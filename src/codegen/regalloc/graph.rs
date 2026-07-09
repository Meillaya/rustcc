// Mirrors nqcc2/lib/backend/regalloc.ml:156-385.

use std::collections::{BTreeMap, BTreeSet};

use crate::codegen::assembly::{Instr, Operand, Reg};
use crate::ir::tacky::TypeEnv;

use super::graph_pseudos::{PseudoNodeContext, add_pseudo_nodes};
use super::liveness::LiveCfg;
use super::operands::{instr_operands, regs_used_and_written};
use super::types::{LivenessConfig, LivenessError, RegisterClass};

pub type NodeId = Operand;
pub type NodeSet = BTreeSet<NodeId>;

#[derive(Clone, Debug, PartialEq)]
pub struct InterferenceNode {
    pub id: NodeId,
    pub neighbors: NodeSet,
    pub spill_cost: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterferenceGraph {
    nodes: BTreeMap<NodeId, InterferenceNode>,
    class: RegisterClass,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InterferenceConfig {
    pub aliased_pseudos: BTreeSet<String>,
    pub static_symbols: BTreeSet<String>,
}

pub struct InterferenceBuild<'a> {
    pub instructions: &'a [Instr],
    pub liveness_cfg: &'a LiveCfg,
    pub class: RegisterClass,
    pub type_env: &'a TypeEnv,
    pub interference: &'a InterferenceConfig,
    pub liveness: &'a LivenessConfig,
}

struct EdgeContext<'a> {
    class: RegisterClass,
    config: &'a LivenessConfig,
}

impl InterferenceGraph {
    pub fn new(class: RegisterClass) -> Self {
        let mut graph = Self {
            nodes: BTreeMap::new(),
            class,
        };
        for reg in class.all_hardregs() {
            graph.add_node(NodeId::Reg(reg), f64::INFINITY);
        }
        let hardregs = graph.node_ids().collect::<Vec<_>>();
        for left in &hardregs {
            for right in &hardregs {
                graph.add_edge(left, right);
            }
        }
        graph
    }

    pub fn class(&self) -> RegisterClass {
        self.class
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: &NodeId) -> Option<&InterferenceNode> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &InterferenceNode> {
        self.nodes.values()
    }

    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().cloned()
    }

    pub fn contains(&self, id: &NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    pub fn neighbors(&self, id: &NodeId) -> Option<&NodeSet> {
        self.nodes.get(id).map(|node| &node.neighbors)
    }

    pub fn degree(&self, id: &NodeId) -> Option<usize> {
        self.neighbors(id).map(BTreeSet::len)
    }

    pub fn add_node(&mut self, id: NodeId, spill_cost: f64) {
        self.nodes
            .entry(id.clone())
            .or_insert_with(|| InterferenceNode {
                id,
                neighbors: NodeSet::new(),
                spill_cost,
            });
    }

    pub fn set_spill_cost(&mut self, id: &NodeId, spill_cost: f64) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.spill_cost = spill_cost;
        }
    }

    pub fn add_edge(&mut self, left: &NodeId, right: &NodeId) {
        if left == right || !self.contains(left) || !self.contains(right) {
            return;
        }
        if let Some(node) = self.nodes.get_mut(left) {
            node.neighbors.insert(right.clone());
        }
        if let Some(node) = self.nodes.get_mut(right) {
            node.neighbors.insert(left.clone());
        }
    }

    pub fn are_neighbors(&self, left: &NodeId, right: &NodeId) -> bool {
        self.nodes
            .get(left)
            .is_some_and(|node| node.neighbors.contains(right))
    }

    pub fn merge_node(&mut self, to_merge: &NodeId, to_keep: &NodeId) {
        let Some(neighbors) = self.neighbors(to_merge).cloned() else {
            return;
        };
        if !self.contains(to_keep) {
            return;
        }
        let merged_spill_cost = self
            .nodes
            .get(to_merge)
            .map(|node| node.spill_cost)
            .unwrap_or(0.0);
        if let Some(kept) = self.nodes.get_mut(to_keep) {
            kept.spill_cost += merged_spill_cost;
        }
        for neighbor in &neighbors {
            self.add_edge(neighbor, to_keep);
        }
        self.nodes.remove(to_merge);
        for neighbor in neighbors {
            if let Some(node) = self.nodes.get_mut(&neighbor) {
                node.neighbors.remove(to_merge);
            }
        }
    }
}

pub fn build_interference(
    input: InterferenceBuild<'_>,
) -> Result<InterferenceGraph, LivenessError> {
    let mut graph = InterferenceGraph::new(input.class);
    let pseudo_context = PseudoNodeContext {
        class: input.class,
        type_env: input.type_env,
        config: input.interference,
    };
    add_pseudo_nodes(&mut graph, input.instructions, &pseudo_context);
    add_spill_costs(&mut graph, input.instructions);
    let edge_context = EdgeContext {
        class: input.class,
        config: input.liveness,
    };
    add_edges(&mut graph, input.liveness_cfg, &edge_context)?;
    Ok(graph)
}

fn add_spill_costs(graph: &mut InterferenceGraph, instructions: &[Instr]) {
    let mut counts = BTreeMap::<String, usize>::new();
    for instr in instructions {
        for op in instr_operands(instr) {
            if let Operand::Pseudo(name) = op {
                *counts.entry(name).or_default() += 1;
            }
        }
    }
    for (name, count) in counts {
        graph.set_spill_cost(&Operand::Pseudo(name), count as f64);
    }
}

fn add_edges(
    graph: &mut InterferenceGraph,
    liveness_cfg: &LiveCfg,
    context: &EdgeContext<'_>,
) -> Result<(), LivenessError> {
    for block in liveness_cfg.blocks() {
        for (live_after_instr, instr) in &block.instructions {
            let updated_regs = regs_used_and_written(instr, context.class, context.config)?.written;
            for live in live_after_instr {
                if move_source_matches_live(instr, live) {
                    continue;
                }
                for updated in &updated_regs {
                    graph.add_edge(live, updated);
                }
            }
        }
    }
    Ok(())
}

fn move_source_matches_live(instr: &Instr, live: &Operand) -> bool {
    match instr {
        Instr::Mov { src, .. }
        | Instr::Movq { src, .. }
        | Instr::MovByte { src, .. }
        | Instr::Movsd { src, .. } => src == live,
        Instr::Movabsq { .. }
        | Instr::Movsx { .. }
        | Instr::MovZeroExtend { .. }
        | Instr::MovSignExtendByte { .. }
        | Instr::MovsdLoad { .. }
        | Instr::Lea { .. }
        | Instr::Cmp { .. }
        | Instr::Cmpq { .. }
        | Instr::CmpDouble { .. }
        | Instr::BinaryOp { .. }
        | Instr::Idiv(_)
        | Instr::Div(_)
        | Instr::Idivq(_)
        | Instr::Divq(_)
        | Instr::Cdq
        | Instr::Cqo
        | Instr::Cltq
        | Instr::Cvtsi2sd { .. }
        | Instr::Cvttsd2si { .. }
        | Instr::Unary { .. }
        | Instr::UnaryQ { .. }
        | Instr::Call(_)
        | Instr::Ret
        | Instr::Push(_)
        | Instr::Pop(_)
        | Instr::Jmp(_)
        | Instr::JmpCC { .. }
        | Instr::SetCC { .. }
        | Instr::Label(_)
        | Instr::AllocateStack(_)
        | Instr::DeallocateStack(_)
        | Instr::Comment(_) => false,
    }
}

pub fn is_hardreg_node(id: &NodeId) -> bool {
    matches!(id, Operand::Reg(_))
}

pub fn hardreg_node(reg: Reg) -> NodeId {
    Operand::Reg(reg)
}
