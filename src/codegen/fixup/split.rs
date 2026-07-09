use crate::codegen::assembly::{BinaryOpInstr, Instr, Operand, Reg};
use crate::codegen::xmm::is_xmm_binary;

/// Split one two-operand x86-64 form into a register-routed pair.
/// Returns a `Vec` because most rules emit exactly one move + one
/// op, but a few emit two moves (for the double-register case).
pub(super) fn split_mem_to_mem(instr: Instr) -> Vec<Instr> {
    match instr {
        // `movl mem, mem` is invalid — route through %r10.
        Instr::Mov {
            src: src @ (Operand::Memory(..) | Operand::Stack(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        // Chapter 11: same split for `movq mem, mem`.
        Instr::Movq {
            src: src @ (Operand::Memory(..) | Operand::Stack(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Movsd {
            src: src @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Movsd {
                src,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::Movsd {
                src: Operand::Reg(Reg::XMM(15)),
                dst,
            },
        ],
        Instr::BinaryOp {
            op: op @ (BinaryOpInstr::Mult | BinaryOpInstr::MultQ),
            src,
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => {
            let scratch_reg = if src == Operand::Reg(Reg::R11) {
                Reg::R10
            } else {
                Reg::R11
            };
            let scratch = Operand::Reg(scratch_reg.clone());
            let load = if matches!(op, BinaryOpInstr::MultQ) {
                Instr::Movq {
                    src: dst.clone(),
                    dst: scratch.clone(),
                }
            } else {
                Instr::Mov {
                    src: dst.clone(),
                    dst: scratch.clone(),
                }
            };
            let store = if matches!(op, BinaryOpInstr::MultQ) {
                Instr::Movq {
                    src: scratch.clone(),
                    dst,
                }
            } else {
                Instr::Mov {
                    src: scratch.clone(),
                    dst,
                }
            };
            let mut out = Vec::new();
            if scratch_reg == Reg::R11 {
                out.push(Instr::Push(Operand::Reg(Reg::R11)));
            }
            out.extend([
                load,
                Instr::BinaryOp {
                    op,
                    src,
                    dst: scratch,
                },
                store,
            ]);
            if scratch_reg == Reg::R11 {
                out.push(Instr::Pop(Reg::R11));
            }
            out
        }
        Instr::BinaryOp {
            op,
            src,
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } if is_xmm_binary(op) => vec![
            Instr::Movsd {
                src: dst.clone(),
                dst: Operand::Reg(Reg::XMM(14)),
            },
            Instr::BinaryOp {
                op,
                src,
                dst: Operand::Reg(Reg::XMM(14)),
            },
            Instr::Movsd {
                src: Operand::Reg(Reg::XMM(14)),
                dst,
            },
        ],
        // `binaryOp op mem, mem` is invalid — route through %r10.
        // Chapter 11: the 64-bit ops (AddQ/SubQ/MultQ/DivQ/RemQ)
        // require a 64-bit scratch move, not the default 32-bit.
        Instr::BinaryOp {
            op,
            src: src @ (Operand::Memory(..) | Operand::Stack(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => {
            let is_wide = matches!(
                op,
                BinaryOpInstr::AddQ
                    | BinaryOpInstr::SubQ
                    | BinaryOpInstr::MultQ
                    | BinaryOpInstr::DivQ
                    | BinaryOpInstr::RemQ
                    | BinaryOpInstr::BitAndQ
                    | BinaryOpInstr::BitOrQ
            );
            let (pre_mov, post_op) = if is_wide {
                (
                    Instr::Movq {
                        src,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::BinaryOp {
                        op,
                        src: Operand::Reg(Reg::R10),
                        dst,
                    },
                )
            } else {
                (
                    Instr::Mov {
                        src,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::BinaryOp {
                        op,
                        src: Operand::Reg(Reg::R10),
                        dst,
                    },
                )
            };
            vec![pre_mov, post_op]
        }
        // `cmpl mem, mem` is invalid — route through %r10.
        Instr::Cmp {
            left,
            right: right @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Mov {
                src: right,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cmp {
                left,
                right: Operand::Reg(Reg::R10),
            },
        ],
        // Chapter 11: same split for `cmpq mem, mem`.
        Instr::Cmpq {
            left,
            right: right @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Movq {
                src: right,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cmpq {
                left,
                right: Operand::Reg(Reg::R10),
            },
        ],
        Instr::CmpDouble {
            left: left @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
            right,
        } => vec![
            Instr::Movsd {
                src: left,
                dst: Operand::Reg(Reg::XMM(14)),
            },
            Instr::CmpDouble {
                left: Operand::Reg(Reg::XMM(14)),
                right,
            },
        ],
        Instr::CmpDouble {
            left,
            right: right @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Movsd {
                src: right,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::CmpDouble {
                left,
                right: Operand::Reg(Reg::XMM(15)),
            },
        ],
        // `idivl mem` is invalid — route through %r10.
        Instr::Idiv(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Idiv(Operand::Reg(Reg::R10)),
        ],
        Instr::Div(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Div(Operand::Reg(Reg::R10)),
        ],
        // Chapter 11: same split for `idivq mem`.
        Instr::Idivq(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Idivq(Operand::Reg(Reg::R10)),
        ],
        Instr::Divq(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Divq(Operand::Reg(Reg::R10)),
        ],
        // Anything else passes through unchanged.
        other => vec![other],
    }
}
