use crate::codegen::assembly::{BinaryOpInstr, Instr, Operand, Reg};
use crate::codegen::replace_pseudos::move_split::split_move_like;
use crate::codegen::xmm::is_xmm_binary;

pub(super) fn split_memory_to_memory(instr: Instr) -> Vec<Instr> {
    if let Some(split) = split_move_like(&instr) {
        return split;
    }
    match instr {
        Instr::BinaryOp {
            op,
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
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
        Instr::BinaryOp {
            op,
            src: src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => {
            // Chapter 11: 64-bit binary ops (AddQ, SubQ, MultQ,
            // BitAnd as longword, etc.) need a 64-bit move to the
            // scratch register, not the default 32-bit `movl`.  We
            // route the wide class through `Movq` and the narrow
            // class through `Mov`.
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
        Instr::Idiv(src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _))) => {
            vec![
                Instr::Mov {
                    src,
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Idiv(Operand::Reg(Reg::R10)),
            ]
        }
        Instr::Div(src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _))) => {
            vec![
                Instr::Mov {
                    src,
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Div(Operand::Reg(Reg::R10)),
            ]
        }
        Instr::Idivq(src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _))) => {
            vec![
                Instr::Movq {
                    src,
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Idivq(Operand::Reg(Reg::R10)),
            ]
        }
        Instr::Divq(src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _))) => {
            vec![
                Instr::Movq {
                    src,
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Divq(Operand::Reg(Reg::R10)),
            ]
        }
        // Chapter 4 + 10: `cmpl mem, mem` is invalid; route the
        // right operand through a scratch register.
        Instr::Cmp {
            left,
            right: right @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
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
        // Chapter 11: same split for the 64-bit `cmpq`.
        Instr::Cmpq {
            left,
            right: right @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
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
            left: left @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
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
            right: right @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
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
        Instr::Cvttsd2si {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => vec![
            Instr::Cvttsd2si {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Cvtsi2sd {
            src: src @ Operand::Imm(_),
            dst: dst @ Operand::Reg(_),
        } => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cvtsi2sd {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Cvtsi2sd {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => vec![
            Instr::Cvtsi2sd {
                src,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::Movsd {
                src: Operand::Reg(Reg::XMM(15)),
                dst,
            },
        ],
        other => vec![other],
    }
}
