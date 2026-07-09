use std::collections::BTreeMap;

use crate::codegen::assembly::{BinaryOpInstr, Instr, Operand, Reg};

use super::types::RegisterClass;

pub(crate) fn replace_colored_pseudos(
    instructions: &[Instr],
    assignments: &BTreeMap<Operand, Option<Reg>>,
    class: RegisterClass,
) -> Vec<Instr> {
    instructions
        .iter()
        .cloned()
        .map(|instr| map_instruction_operands(instr, assignments, class))
        .collect()
}

pub(crate) fn cleanup_redundant_moves(instructions: Vec<Instr>) -> Vec<Instr> {
    instructions
        .into_iter()
        .filter(|instr| !is_redundant_move(instr))
        .collect()
}

fn is_redundant_move(instr: &Instr) -> bool {
    match instr {
        Instr::Mov { src, dst }
        | Instr::Movq { src, dst }
        | Instr::MovByte { src, dst }
        | Instr::Movsd { src, dst } => src == dst,
        _ => false,
    }
}

fn replace_operand(op: Operand, assignments: &BTreeMap<Operand, Option<Reg>>) -> Operand {
    if let Operand::Pseudo(_) = &op
        && let Some(Some(reg)) = assignments.get(&op)
    {
        return Operand::Reg(reg.clone());
    }
    op
}

fn map_instruction_operands(
    instr: Instr,
    assignments: &BTreeMap<Operand, Option<Reg>>,
    class: RegisterClass,
) -> Instr {
    let map = |op| replace_operand(op, assignments);
    match (class, instr) {
        (RegisterClass::Gp, Instr::Mov { src, dst }) => Instr::Mov {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::Movq { src, dst }) => Instr::Movq {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Gp, Instr::MovByte { src, dst }) => Instr::MovByte {
            src: map(src),
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
        (RegisterClass::Gp, Instr::Movabsq { src, dst }) => Instr::Movabsq { src, dst: map(dst) },
        (RegisterClass::Gp, Instr::MovsdLoad { src, dst }) => {
            Instr::MovsdLoad { src, dst: map(dst) }
        }
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
        (RegisterClass::Gp, Instr::BinaryOp { op, src, dst }) if !is_xmm_binary(op) => {
            Instr::BinaryOp {
                op,
                src: map(src),
                dst: map(dst),
            }
        }
        (RegisterClass::Gp, Instr::Cvtsi2sd { src, dst }) => Instr::Cvtsi2sd { src: map(src), dst },
        (RegisterClass::Gp, Instr::Cvttsd2si { src, dst }) => {
            Instr::Cvttsd2si { src, dst: map(dst) }
        }
        (RegisterClass::Gp, Instr::Unary { op, operand }) => Instr::Unary {
            op,
            operand: map(operand),
        },
        (RegisterClass::Gp, Instr::UnaryQ { op, operand }) => Instr::UnaryQ {
            op,
            operand: map(operand),
        },
        (RegisterClass::Gp, Instr::Idiv(op)) => Instr::Idiv(map(op)),
        (RegisterClass::Gp, Instr::Div(op)) => Instr::Div(map(op)),
        (RegisterClass::Gp, Instr::Idivq(op)) => Instr::Idivq(map(op)),
        (RegisterClass::Gp, Instr::Divq(op)) => Instr::Divq(map(op)),
        (RegisterClass::Gp, Instr::Push(op)) => Instr::Push(map(op)),
        (RegisterClass::Gp, Instr::SetCC { cc, dst }) => Instr::SetCC { cc, dst: map(dst) },

        (RegisterClass::Xmm, Instr::Movsd { src, dst }) => Instr::Movsd {
            src: map(src),
            dst: map(dst),
        },
        (RegisterClass::Xmm, Instr::CmpDouble { left, right }) => Instr::CmpDouble {
            left: map(left),
            right: map(right),
        },
        (RegisterClass::Xmm, Instr::BinaryOp { op, src, dst }) if is_xmm_binary(op) => {
            Instr::BinaryOp {
                op,
                src: map(src),
                dst: map(dst),
            }
        }
        (RegisterClass::Xmm, Instr::Cvtsi2sd { src, dst }) => {
            Instr::Cvtsi2sd { src, dst: map(dst) }
        }
        (RegisterClass::Xmm, Instr::Cvttsd2si { src, dst }) => {
            Instr::Cvttsd2si { src: map(src), dst }
        }
        (RegisterClass::Xmm, other) | (RegisterClass::Gp, other) => other,
    }
}

fn is_xmm_binary(op: BinaryOpInstr) -> bool {
    matches!(
        op,
        BinaryOpInstr::AddDouble
            | BinaryOpInstr::SubDouble
            | BinaryOpInstr::MultDouble
            | BinaryOpInstr::SseDivDouble
            | BinaryOpInstr::XorDouble
    )
}
