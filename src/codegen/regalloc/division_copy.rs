use crate::codegen::assembly::{Instr, Operand, Reg};

pub(super) fn cleanup_destructive_dividend_copies(instructions: Vec<Instr>) -> Vec<Instr> {
    let mut output = Vec::with_capacity(instructions.len());
    let mut index = 0;
    while index < instructions.len() {
        if let Some((replacement, skip)) = dividend_copy_replacement(&instructions, index) {
            output.push(replacement);
            index += skip;
        } else {
            output.push(instructions[index].clone());
            index += 1;
        }
    }
    output
}

fn dividend_copy_replacement(instructions: &[Instr], index: usize) -> Option<(Instr, usize)> {
    let (src, saved_reg, wide) = saved_dividend_copy(instructions.get(index)?)?;
    if !matches_saved_to_ax(instructions.get(index + 1)?, &saved_reg, wide) {
        return None;
    }
    if instruction_mentions_reg(instructions.get(index + 2)?, &saved_reg) {
        return None;
    }
    if !matches_extension(instructions.get(index + 3)?, wide)
        || !matches_division(instructions.get(index + 4)?, wide)
        || !matches_ax_to_saved(instructions.get(index + 5)?, &saved_reg, wide)
    {
        return None;
    }
    Some((build_ax_copy(src.clone(), wide), 2))
}

fn saved_dividend_copy(instr: &Instr) -> Option<(&Operand, Reg, bool)> {
    match instr {
        Instr::Mov {
            src,
            dst: Operand::Reg(reg),
        } => Some((src, reg.clone(), false)),
        Instr::Movq {
            src,
            dst: Operand::Reg(reg),
        } => Some((src, reg.clone(), true)),
        _ => None,
    }
}

fn matches_saved_to_ax(instr: &Instr, saved_reg: &Reg, wide: bool) -> bool {
    match (wide, instr) {
        (
            false,
            Instr::Mov {
                src: Operand::Reg(src),
                dst: Operand::Reg(Reg::AX),
            },
        ) => src == saved_reg,
        (
            true,
            Instr::Movq {
                src: Operand::Reg(src),
                dst: Operand::Reg(Reg::AX),
            },
        ) => src == saved_reg,
        _ => false,
    }
}

fn matches_ax_to_saved(instr: &Instr, saved_reg: &Reg, wide: bool) -> bool {
    match (wide, instr) {
        (
            false,
            Instr::Mov {
                src: Operand::Reg(Reg::AX),
                dst: Operand::Reg(dst),
            },
        ) => dst == saved_reg,
        (
            true,
            Instr::Movq {
                src: Operand::Reg(Reg::AX),
                dst: Operand::Reg(dst),
            },
        ) => dst == saved_reg,
        _ => false,
    }
}

fn matches_extension(instr: &Instr, wide: bool) -> bool {
    matches!((wide, instr), (false, Instr::Cdq) | (true, Instr::Cqo))
}

fn matches_division(instr: &Instr, wide: bool) -> bool {
    matches!(
        (wide, instr),
        (false, Instr::Idiv(_) | Instr::Div(_)) | (true, Instr::Idivq(_) | Instr::Divq(_))
    )
}

fn build_ax_copy(src: Operand, wide: bool) -> Instr {
    if wide {
        Instr::Movq {
            src,
            dst: Operand::Reg(Reg::AX),
        }
    } else {
        Instr::Mov {
            src,
            dst: Operand::Reg(Reg::AX),
        }
    }
}

fn instruction_mentions_reg(instr: &Instr, reg: &Reg) -> bool {
    explicit_operands(instr)
        .into_iter()
        .any(|operand| operand_mentions_reg(&operand, reg))
}

fn explicit_operands(instr: &Instr) -> Vec<Operand> {
    match instr {
        Instr::Mov { src, dst }
        | Instr::Movq { src, dst }
        | Instr::MovByte { src, dst }
        | Instr::Movsx { src, dst }
        | Instr::MovZeroExtend { src, dst }
        | Instr::MovSignExtendByte { src, dst }
        | Instr::Movsd { src, dst }
        | Instr::Lea { src, dst }
        | Instr::Cvtsi2sd { src, dst }
        | Instr::Cvttsd2si { src, dst } => vec![src.clone(), dst.clone()],
        Instr::Movabsq { dst, .. } | Instr::MovsdLoad { dst, .. } => vec![dst.clone()],
        Instr::BinaryOp { src, dst, .. } => vec![src.clone(), dst.clone()],
        Instr::Unary { operand, .. } | Instr::UnaryQ { operand, .. } => vec![operand.clone()],
        Instr::Cmp { left, right }
        | Instr::Cmpq { left, right }
        | Instr::CmpDouble { left, right } => vec![left.clone(), right.clone()],
        Instr::Idiv(op) | Instr::Div(op) | Instr::Idivq(op) | Instr::Divq(op) | Instr::Push(op) => {
            vec![op.clone()]
        }
        Instr::SetCC { dst, .. } => vec![dst.clone()],
        Instr::Pop(reg) => vec![Operand::Reg(reg.clone())],
        Instr::Cdq
        | Instr::Cqo
        | Instr::Cltq
        | Instr::Call(_)
        | Instr::Ret
        | Instr::Jmp(_)
        | Instr::JmpCC { .. }
        | Instr::Label(_)
        | Instr::AllocateStack(_)
        | Instr::DeallocateStack(_)
        | Instr::Comment(_) => Vec::new(),
    }
}

fn operand_mentions_reg(operand: &Operand, reg: &Reg) -> bool {
    match operand {
        Operand::Reg(candidate) | Operand::Memory(candidate, _) => candidate == reg,
        Operand::MemoryIndexed(base, index, _) => base == reg || index == reg,
        Operand::Imm(_)
        | Operand::Pseudo(_)
        | Operand::PseudoMem(_, _)
        | Operand::Stack(_)
        | Operand::Data(_)
        | Operand::DataOffset(_, _) => false,
    }
}
