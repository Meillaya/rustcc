use crate::codegen::assembly::{Instr, Operand, Reg};

pub(super) fn split_move_like(instr: &Instr) -> Option<Vec<Instr>> {
    match instr.clone() {
        // Chapter 10: route every memory-to-memory `movl` through
        // `%r10d` so `mov src, dst` works when both operands are
        // stack slots or RIP-relative data references.
        Instr::Mov {
            src: src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => Some(vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        // Chapter 11: same split for the 64-bit `movq`.
        Instr::Movq {
            src: src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
            dst:
                dst @ (Operand::Stack(_)
                | Operand::Data(_)
                | Operand::DataOffset(_, _)
                | Operand::Memory(_, _)
                | Operand::MemoryIndexed(_, _, _)),
        } => Some(vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        Instr::MovByte {
            src:
                src @ (Operand::Stack(_)
                | Operand::Data(_)
                | Operand::DataOffset(_, _)
                | Operand::Memory(_, _)
                | Operand::MemoryIndexed(_, _, _)),
            dst:
                dst @ (Operand::Stack(_)
                | Operand::Data(_)
                | Operand::DataOffset(_, _)
                | Operand::Memory(_, _)
                | Operand::MemoryIndexed(_, _, _)),
        } => Some(vec![
            Instr::MovByte {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::MovByte {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        Instr::Movsd {
            src: src @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => Some(vec![
            Instr::Movsd {
                src,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::Movsd {
                src: Operand::Reg(Reg::XMM(15)),
                dst,
            },
        ]),
        // Chapter 11: `movslq` requires a register destination
        // (x86-64 forbids memory destinations for sign-extending
        // moves).  Route through `%r10` whenever the destination is
        // a stack slot or a RIP-relative data reference.
        Instr::Movsx {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => Some(vec![
            Instr::Movsx {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        Instr::MovZeroExtend {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => Some(vec![
            Instr::MovZeroExtend {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        Instr::MovSignExtendByte {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => Some(vec![
            Instr::MovSignExtendByte {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        Instr::Lea {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_) | Operand::DataOffset(_, _)),
        } => Some(vec![
            Instr::Lea {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ]),
        _ => None,
    }
}
