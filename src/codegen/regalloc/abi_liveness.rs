use std::collections::{BTreeMap, BTreeSet};

use crate::codegen::assembly::{Instr, Operand, Reg};

use super::types::LivenessConfig;

pub(super) fn conservative_liveness_config(instructions: &[Instr]) -> LivenessConfig {
    let mut call_param_regs = BTreeMap::<String, BTreeSet<Reg>>::new();
    for (index, instr) in instructions.iter().enumerate() {
        if let Instr::Call(name) = instr {
            call_param_regs
                .entry(name.clone())
                .or_default()
                .extend(call_regs_before(instructions, index));
        }
    }
    LivenessConfig {
        return_regs: return_regs_before_rets(instructions).into_iter().collect(),
        call_param_regs: call_param_regs
            .into_iter()
            .map(|(name, regs)| (name, regs.into_iter().collect()))
            .collect(),
    }
}

fn call_regs_before(instructions: &[Instr], call_index: usize) -> BTreeSet<Reg> {
    let mut regs = BTreeSet::new();
    for instr in instructions[..call_index].iter().rev() {
        match instr {
            Instr::Call(_) | Instr::Ret | Instr::Jmp(_) | Instr::JmpCC { .. } | Instr::Label(_) => {
                break;
            }
            _ => {
                let Some(reg) = written_reg(instr) else {
                    continue;
                };
                if param_regs().contains(&reg) {
                    regs.insert(reg);
                }
            }
        }
    }
    regs
}

fn return_regs_before_rets(instructions: &[Instr]) -> BTreeSet<Reg> {
    let mut regs = BTreeSet::new();
    for (index, instr) in instructions.iter().enumerate() {
        if !matches!(instr, Instr::Ret) {
            continue;
        }
        for prior in instructions[..index].iter().rev() {
            let Some(reg) = written_reg(prior) else {
                break;
            };
            if !return_regs().contains(&reg) {
                break;
            }
            regs.insert(reg);
        }
    }
    regs
}

fn written_reg(instr: &Instr) -> Option<Reg> {
    match instr {
        Instr::Mov { dst, .. }
        | Instr::Movq { dst, .. }
        | Instr::MovByte { dst, .. }
        | Instr::Movsx { dst, .. }
        | Instr::MovZeroExtend { dst, .. }
        | Instr::MovSignExtendByte { dst, .. }
        | Instr::Movsd { dst, .. }
        | Instr::MovsdLoad { dst, .. }
        | Instr::Lea { dst, .. }
        | Instr::Cvtsi2sd { dst, .. }
        | Instr::Cvttsd2si { dst, .. }
        | Instr::SetCC { dst, .. } => match dst {
            Operand::Reg(reg) => Some(reg.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn param_regs() -> BTreeSet<Reg> {
    [
        Reg::DI,
        Reg::SI,
        Reg::DX,
        Reg::CX,
        Reg::R8,
        Reg::R9,
        Reg::XMM(0),
        Reg::XMM(1),
        Reg::XMM(2),
        Reg::XMM(3),
        Reg::XMM(4),
        Reg::XMM(5),
        Reg::XMM(6),
        Reg::XMM(7),
    ]
    .into_iter()
    .collect()
}

fn return_regs() -> BTreeSet<Reg> {
    [Reg::AX, Reg::DX, Reg::XMM(0), Reg::XMM(1)]
        .into_iter()
        .collect()
}
