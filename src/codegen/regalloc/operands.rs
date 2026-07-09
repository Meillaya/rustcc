// Mirrors nqcc2/lib/backend/regalloc.ml:24-85.

use std::collections::BTreeSet;

use crate::codegen::assembly::{Instr, Operand, Reg};

use super::types::{LiveSet, LivenessConfig, LivenessError, RegisterClass, regs_to_operands};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UseDef {
    pub used: LiveSet,
    pub written: LiveSet,
}

pub fn regs_used_and_written(
    instr: &Instr,
    class: RegisterClass,
    config: &LivenessConfig,
) -> Result<UseDef, LivenessError> {
    let (used_ops, written_ops) = raw_operands(instr, class, config)?;
    let mut used = used_ops
        .into_iter()
        .flat_map(regs_read)
        .collect::<LiveSet>();
    let mut written = LiveSet::new();
    for op in written_ops {
        let (read, write) = regs_read_or_written(op);
        used.extend(read);
        written.extend(write);
    }
    Ok(UseDef { used, written })
}

pub fn instr_operands(instr: &Instr) -> Vec<Operand> {
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
        Instr::Cdq
        | Instr::Cqo
        | Instr::Cltq
        | Instr::Call(_)
        | Instr::Ret
        | Instr::Jmp(_)
        | Instr::JmpCC { .. }
        | Instr::Label(_)
        | Instr::Pop(_)
        | Instr::AllocateStack(_)
        | Instr::DeallocateStack(_)
        | Instr::Comment(_) => Vec::new(),
    }
}

fn raw_operands(
    instr: &Instr,
    class: RegisterClass,
    config: &LivenessConfig,
) -> Result<(Vec<Operand>, Vec<Operand>), LivenessError> {
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
        | Instr::Cvttsd2si { src, dst } => Ok((vec![src.clone()], vec![dst.clone()])),
        Instr::Movabsq { dst, .. } | Instr::MovsdLoad { dst, .. } => {
            Ok((Vec::new(), vec![dst.clone()]))
        }
        Instr::BinaryOp { src, dst, .. } => Ok((vec![src.clone(), dst.clone()], vec![dst.clone()])),
        Instr::Unary { operand, .. } | Instr::UnaryQ { operand, .. } => {
            Ok((vec![operand.clone()], vec![operand.clone()]))
        }
        Instr::Cmp { left, right }
        | Instr::Cmpq { left, right }
        | Instr::CmpDouble { left, right } => Ok((vec![left.clone(), right.clone()], Vec::new())),
        Instr::Idiv(op) | Instr::Div(op) | Instr::Idivq(op) | Instr::Divq(op) => Ok((
            vec![op.clone(), Operand::Reg(Reg::AX), Operand::Reg(Reg::DX)],
            vec![Operand::Reg(Reg::AX), Operand::Reg(Reg::DX)],
        )),
        Instr::Cdq | Instr::Cqo => Ok((vec![Operand::Reg(Reg::AX)], vec![Operand::Reg(Reg::DX)])),
        Instr::Cltq => Ok((vec![Operand::Reg(Reg::AX)], vec![Operand::Reg(Reg::AX)])),
        Instr::SetCC { dst, .. } => Ok((Vec::new(), vec![dst.clone()])),
        Instr::Push(op) => Ok((vec![op.clone()], Vec::new())),
        Instr::Call(name) => {
            let all_hardregs = class.all_hardregs();
            let used = config
                .call_param_regs
                .get(name)
                .ok_or_else(|| LivenessError::MissingCallMetadata {
                    callee: name.clone(),
                })?
                .iter()
                .filter(|reg| all_hardregs.contains(reg))
                .cloned()
                .map(Operand::Reg)
                .collect();
            let written = regs_to_operands(&class.caller_saved_regs())
                .into_iter()
                .collect();
            Ok((used, written))
        }
        Instr::Pop(reg) => Ok((Vec::new(), vec![Operand::Reg(reg.clone())])),
        Instr::Ret
        | Instr::Jmp(_)
        | Instr::JmpCC { .. }
        | Instr::Label(_)
        | Instr::AllocateStack(_)
        | Instr::DeallocateStack(_)
        | Instr::Comment(_) => Ok((Vec::new(), Vec::new())),
    }
}

fn regs_read(op: Operand) -> LiveSet {
    match op {
        Operand::Pseudo(_) | Operand::Reg(_) => BTreeSet::from([op]),
        Operand::Memory(reg, _) => BTreeSet::from([Operand::Reg(reg)]),
        Operand::MemoryIndexed(base, index, _) => {
            BTreeSet::from([Operand::Reg(base), Operand::Reg(index)])
        }
        Operand::Imm(_)
        | Operand::PseudoMem(_, _)
        | Operand::Stack(_)
        | Operand::Data(_)
        | Operand::DataOffset(_, _) => LiveSet::new(),
    }
}

fn regs_read_or_written(op: Operand) -> (LiveSet, LiveSet) {
    match op {
        Operand::Pseudo(_) | Operand::Reg(_) => (LiveSet::new(), BTreeSet::from([op])),
        Operand::Memory(reg, _) => (BTreeSet::from([Operand::Reg(reg)]), LiveSet::new()),
        Operand::MemoryIndexed(base, index, _) => (
            BTreeSet::from([Operand::Reg(base), Operand::Reg(index)]),
            LiveSet::new(),
        ),
        Operand::Imm(_)
        | Operand::PseudoMem(_, _)
        | Operand::Stack(_)
        | Operand::Data(_)
        | Operand::DataOffset(_, _) => (LiveSet::new(), LiveSet::new()),
    }
}
