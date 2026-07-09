// Mirrors nqcc2/lib/backend/regalloc.ml:516-563.

use std::collections::{BTreeMap, BTreeSet};

use crate::codegen::assembly::{Operand, Reg};

use super::graph::{InterferenceGraph, NodeId};
use super::simplify::{Simplification, SimplifyStep};

pub type ColorMap = BTreeMap<NodeId, Option<Reg>>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectResult {
    pub assignments: ColorMap,
    pub used_callee_saved_regs: BTreeSet<Reg>,
}

struct ColorContext {
    color_to_reg: BTreeMap<usize, Reg>,
    reg_to_color: BTreeMap<Reg, usize>,
    caller_saved: BTreeSet<Reg>,
}

pub fn select(graph: &InterferenceGraph, simplification: &Simplification) -> SelectResult {
    let context = ColorContext::new(
        graph.class().all_hardregs(),
        graph.class().caller_saved_regs(),
    );
    let mut pseudo_colors = BTreeMap::<NodeId, usize>::new();
    let mut assignments = ColorMap::new();
    let mut used_callee_saved_regs = BTreeSet::new();

    for step in simplification.stack.iter().rev() {
        let assignment = color_node(graph, step, &context, &pseudo_colors);
        if let Some(reg) = assignment.clone() {
            if !context.caller_saved.contains(&reg) {
                used_callee_saved_regs.insert(reg.clone());
            }
            if let Some(color) = context.reg_to_color.get(&reg) {
                pseudo_colors.insert(step.node.clone(), *color);
            }
        }
        assignments.insert(step.node.clone(), assignment);
    }

    SelectResult {
        assignments,
        used_callee_saved_regs,
    }
}

impl ColorContext {
    fn new(hardregs: Vec<Reg>, caller_saved: Vec<Reg>) -> Self {
        let caller_saved = caller_saved.into_iter().collect::<BTreeSet<_>>();
        let mut reg_to_color = BTreeMap::new();
        let mut available = (0..hardregs.len()).collect::<BTreeSet<_>>();
        for reg in hardregs.iter().rev() {
            let color = choose_hardreg_color(reg, &caller_saved, &available);
            available.remove(&color);
            reg_to_color.insert(reg.clone(), color);
        }
        let color_to_reg = reg_to_color
            .iter()
            .map(|(reg, color)| (*color, reg.clone()))
            .collect();
        Self {
            color_to_reg,
            reg_to_color,
            caller_saved,
        }
    }
}

fn choose_hardreg_color(
    reg: &Reg,
    caller_saved: &BTreeSet<Reg>,
    available: &BTreeSet<usize>,
) -> usize {
    if caller_saved.contains(reg) {
        available.first().copied().unwrap_or(0)
    } else {
        available.last().copied().unwrap_or(0)
    }
}

fn color_node(
    graph: &InterferenceGraph,
    step: &SimplifyStep,
    context: &ColorContext,
    pseudo_colors: &BTreeMap<NodeId, usize>,
) -> Option<Reg> {
    let mut available = context
        .color_to_reg
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    if let Some(neighbors) = graph.neighbors(&step.node) {
        for neighbor in neighbors {
            remove_neighbor_color(neighbor, &mut available, context, pseudo_colors);
        }
    }
    available
        .into_iter()
        .next()
        .and_then(|color| context.color_to_reg.get(&color).cloned())
}

fn remove_neighbor_color(
    neighbor: &NodeId,
    available: &mut BTreeSet<usize>,
    context: &ColorContext,
    pseudo_colors: &BTreeMap<NodeId, usize>,
) {
    match neighbor {
        Operand::Reg(reg) => {
            if let Some(color) = context.reg_to_color.get(reg) {
                available.remove(color);
            }
        }
        Operand::Pseudo(_) => {
            if let Some(color) = pseudo_colors.get(neighbor) {
                available.remove(color);
            }
        }
        Operand::Imm(_)
        | Operand::Memory(_, _)
        | Operand::MemoryIndexed(_, _, _)
        | Operand::PseudoMem(_, _)
        | Operand::Stack(_)
        | Operand::Data(_)
        | Operand::DataOffset(_, _) => {}
    }
}
