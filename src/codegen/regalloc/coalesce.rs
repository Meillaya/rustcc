// Mirrors nqcc2/lib/backend/regalloc.ml:388-479.

use std::collections::BTreeMap;

use crate::codegen::assembly::{Instr, Operand};
use crate::codegen::xmm::is_xmm_binary;

use super::graph::{InterferenceGraph, NodeId, is_hardreg_node};
use super::types::RegisterClass;

type CoalescedRegs = BTreeMap<NodeId, NodeId>;

pub(crate) fn coalesce_once(
    mut graph: InterferenceGraph,
    instructions: &[Instr],
    class: RegisterClass,
) -> (InterferenceGraph, Vec<Instr>, bool) {
    let coalesced_regs = coalesce(&mut graph, instructions);
    if coalesced_regs.is_empty() {
        return (graph, instructions.to_vec(), false);
    }
    let rewritten = rewrite_coalesced(instructions, &coalesced_regs, class);
    (graph, rewritten, true)
}

fn coalesce(graph: &mut InterferenceGraph, instructions: &[Instr]) -> CoalescedRegs {
    let mut reg_map = CoalescedRegs::new();
    for instr in instructions {
        let Some((src, dst)) = copy_operands(instr) else {
            continue;
        };
        let src_root = find(src, &reg_map);
        let dst_root = find(dst, &reg_map);
        if !graph.contains(&src_root)
            || !graph.contains(&dst_root)
            || src_root == dst_root
            || graph.are_neighbors(&src_root, &dst_root)
            || !conservative_coalescable(graph, &src_root, &dst_root)
        {
            continue;
        }
        if is_hardreg_node(&src_root) {
            graph.merge_node(&dst_root, &src_root);
            reg_map.insert(dst_root, src_root);
        } else {
            graph.merge_node(&src_root, &dst_root);
            reg_map.insert(src_root, dst_root);
        }
    }
    reg_map
}

fn conservative_coalescable(graph: &InterferenceGraph, src: &NodeId, dst: &NodeId) -> bool {
    if briggs_test(graph, src, dst) {
        return true;
    }
    match (src, dst) {
        (Operand::Reg(_), Operand::Pseudo(_)) => george_test(graph, src, dst),
        (Operand::Pseudo(_), Operand::Reg(_)) => george_test(graph, dst, src),
        _ => false,
    }
}

fn george_test(graph: &InterferenceGraph, hardreg: &NodeId, pseudo: &NodeId) -> bool {
    let Some(neighbors) = graph.neighbors(pseudo) else {
        return false;
    };
    let k = graph.class().all_hardregs().len();
    neighbors.iter().all(|neighbor| {
        graph.are_neighbors(neighbor, hardreg) || graph.degree(neighbor).unwrap_or(0) < k
    })
}

fn briggs_test(graph: &InterferenceGraph, left: &NodeId, right: &NodeId) -> bool {
    let k = graph.class().all_hardregs().len();
    let mut neighbors = graph.neighbors(left).cloned().unwrap_or_default();
    neighbors.extend(graph.neighbors(right).cloned().unwrap_or_default());
    let significant = neighbors
        .iter()
        .filter(|neighbor| {
            let degree = graph.degree(neighbor).unwrap_or(0);
            let adjusted =
                if graph.are_neighbors(left, neighbor) && graph.are_neighbors(right, neighbor) {
                    degree.saturating_sub(1)
                } else {
                    degree
                };
            adjusted >= k
        })
        .count();
    significant < k
}

fn rewrite_coalesced(
    instructions: &[Instr],
    coalesced_regs: &CoalescedRegs,
    class: RegisterClass,
) -> Vec<Instr> {
    instructions
        .iter()
        .filter_map(|instr| rewrite_instruction(instr, coalesced_regs, class))
        .collect()
}

fn rewrite_instruction(
    instr: &Instr,
    coalesced_regs: &CoalescedRegs,
    class: RegisterClass,
) -> Option<Instr> {
    let map = |op: &Operand| find(op, coalesced_regs);
    match (class, instr) {
        (RegisterClass::Gp, Instr::Mov { src, dst }) => {
            rewrite_copy(src, dst, |src, dst| Instr::Mov { src, dst }, &map)
        }
        (RegisterClass::Gp, Instr::Movq { src, dst }) => {
            rewrite_copy(src, dst, |src, dst| Instr::Movq { src, dst }, &map)
        }
        (RegisterClass::Gp, Instr::MovByte { src, dst }) => {
            rewrite_copy(src, dst, |src, dst| Instr::MovByte { src, dst }, &map)
        }
        (RegisterClass::Xmm, Instr::Movsd { src, dst }) => {
            rewrite_copy(src, dst, |src, dst| Instr::Movsd { src, dst }, &map)
        }
        _ => Some(map_instruction_operands(instr, class, &map)),
    }
}

fn rewrite_copy(
    src: &Operand,
    dst: &Operand,
    build: impl FnOnce(Operand, Operand) -> Instr,
    map: &impl Fn(&Operand) -> Operand,
) -> Option<Instr> {
    let new_src = map(src);
    let new_dst = map(dst);
    (new_src != new_dst).then(|| build(new_src, new_dst))
}

fn find(op: &Operand, reg_map: &CoalescedRegs) -> Operand {
    let mut current = op.clone();
    while let Some(next) = reg_map.get(&current) {
        current = next.clone();
    }
    current
}

fn copy_operands(instr: &Instr) -> Option<(&Operand, &Operand)> {
    match instr {
        Instr::Mov { src, dst }
        | Instr::Movq { src, dst }
        | Instr::MovByte { src, dst }
        | Instr::Movsd { src, dst } => Some((src, dst)),
        _ => None,
    }
}

fn map_instruction_operands(
    instr: &Instr,
    class: RegisterClass,
    map: &impl Fn(&Operand) -> Operand,
) -> Instr {
    match (class, instr) {
        (RegisterClass::Gp, Instr::Movabsq { src, dst }) => Instr::Movabsq {
            src: *src,
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::Movsx { src, dst }) => Instr::Movsx {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::MovZeroExtend { src, dst }) => Instr::MovZeroExtend {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::MovSignExtendByte { src, dst }) => Instr::MovSignExtendByte {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::MovsdLoad { src, dst }) => Instr::MovsdLoad {
            src: src.clone(),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::Lea { src, dst }) => Instr::Lea {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::Cmp { left, right }) => Instr::Cmp {
            left: map(left),
            right: map(right),
        },
        (RegisterClass::Gp, Instr::Cmpq { left, right }) => Instr::Cmpq {
            left: map(left),
            right: map(right),
        },
        (RegisterClass::Gp, Instr::BinaryOp { op, src, dst }) if !is_xmm_binary(*op) => {
            Instr::BinaryOp {
                op: *op,
                src: map(src),
                dst: map(dst),
            }
        }
        (RegisterClass::Gp, Instr::Idiv(op)) => Instr::Idiv(map(op)),
        (RegisterClass::Gp, Instr::Div(op)) => Instr::Div(map(op)),
        (RegisterClass::Gp, Instr::Idivq(op)) => Instr::Idivq(map(op)),
        (RegisterClass::Gp, Instr::Divq(op)) => Instr::Divq(map(op)),
        (RegisterClass::Gp, Instr::Cvtsi2sd { src, dst }) => Instr::Cvtsi2sd {
            src: map(src),
            dst: dst.clone(),
        },
        (RegisterClass::Gp, Instr::Cvttsd2si { src, dst }) => Instr::Cvttsd2si {
            src: src.clone(),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::Unary { op, operand }) => Instr::Unary {
            op: *op,
            operand: map(operand),
        },
        (RegisterClass::Gp, Instr::UnaryQ { op, operand }) => Instr::UnaryQ {
            op: *op,
            operand: map(operand),
        },
        (RegisterClass::Gp, Instr::Push(op)) => Instr::Push(map(op)),
        (RegisterClass::Gp, Instr::SetCC { cc, dst }) => Instr::SetCC {
            cc: *cc,
            dst: map(dst),
        },
        (RegisterClass::Xmm, Instr::CmpDouble { left, right }) => Instr::CmpDouble {
            left: map(left),
            right: map(right),
        },
        (RegisterClass::Xmm, Instr::BinaryOp { op, src, dst }) if is_xmm_binary(*op) => {
            Instr::BinaryOp {
                op: *op,
                src: map(src),
                dst: map(dst),
            }
        }
        (RegisterClass::Xmm, Instr::Cvtsi2sd { src, dst }) => Instr::Cvtsi2sd {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Xmm, Instr::Cvttsd2si { src, dst }) => Instr::Cvttsd2si {
            src: map(src),
            dst: dst.clone(),
        },
        (RegisterClass::Xmm, Instr::MovsdLoad { src, dst }) => Instr::MovsdLoad {
            src: src.clone(),
            dst: map(dst),
        },
        (RegisterClass::Gp, other) | (RegisterClass::Xmm, other) => other.clone(),
    }
}
