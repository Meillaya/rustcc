// Mirrors nqcc2/lib/backend/regalloc.ml:470-516.

use std::collections::BTreeSet;

use crate::codegen::assembly::Operand;

use super::graph::{InterferenceGraph, NodeId, is_hardreg_node};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimplifyChoice {
    LowDegree,
    SpillCandidate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplifyStep {
    pub node: NodeId,
    pub degree: usize,
    pub choice: SimplifyChoice,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Simplification {
    pub stack: Vec<SimplifyStep>,
}

pub fn simplify(graph: &InterferenceGraph) -> Simplification {
    let k = graph.class().all_hardregs().len();
    let mut remaining = graph
        .node_ids()
        .filter(|id| !is_hardreg_node(id))
        .collect::<BTreeSet<_>>();
    let mut stack = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let Some((node, degree, choice)) = choose_next(graph, &remaining, k) else {
            break;
        };
        remaining.remove(&node);
        stack.push(SimplifyStep {
            node,
            degree,
            choice,
        });
    }

    Simplification { stack }
}

fn choose_next(
    graph: &InterferenceGraph,
    remaining: &BTreeSet<NodeId>,
    k: usize,
) -> Option<(NodeId, usize, SimplifyChoice)> {
    let mut candidates = remaining.iter().map(|id| {
        let degree = active_degree(graph, id, remaining);
        (id.clone(), degree)
    });
    if let Some((node, degree)) = candidates.find(|(_, degree)| *degree < k) {
        return Some((node, degree, SimplifyChoice::LowDegree));
    }
    remaining
        .iter()
        .filter_map(|id| spill_metric(graph, id, remaining).map(|metric| (id.clone(), metric)))
        .min_by(|(left_id, left_metric), (right_id, right_metric)| {
            left_metric
                .total_cmp(right_metric)
                .then_with(|| left_id.cmp(right_id))
        })
        .map(|(node, _)| {
            let degree = active_degree(graph, &node, remaining);
            (node, degree, SimplifyChoice::SpillCandidate)
        })
}

fn active_degree(graph: &InterferenceGraph, id: &NodeId, remaining: &BTreeSet<NodeId>) -> usize {
    graph.neighbors(id).map_or(0, |neighbors| {
        neighbors
            .iter()
            .filter(|neighbor| match neighbor {
                Operand::Reg(_) => true,
                Operand::Pseudo(_) => remaining.contains(*neighbor),
                Operand::Imm(_)
                | Operand::Memory(_, _)
                | Operand::MemoryIndexed(_, _, _)
                | Operand::PseudoMem(_, _)
                | Operand::Stack(_)
                | Operand::Data(_)
                | Operand::DataOffset(_, _) => false,
            })
            .count()
    })
}

fn spill_metric(
    graph: &InterferenceGraph,
    id: &NodeId,
    remaining: &BTreeSet<NodeId>,
) -> Option<f64> {
    let node = graph.node(id)?;
    let degree = active_degree(graph, id, remaining).max(1) as f64;
    Some(node.spill_cost / degree)
}
