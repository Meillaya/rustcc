//! Instruction classification for the generic CFG builder.
//!
//! Mirrors `nqcc2/lib/cfg.ml:5-19` and the `TackyCfg` / `AsmCfg` functor
//! instantiations at `nqcc2/lib/cfg.ml:274-341`.

use crate::codegen::assembly;
use crate::ir::tacky;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimpleInstr<'a> {
    Label(&'a str),
    ConditionalJump(&'a str),
    UnconditionalJump(&'a str),
    Return,
    Other,
}

pub trait CfgInstruction {
    fn simplify(&self) -> SimpleInstr<'_>;
}

impl CfgInstruction for tacky::Instruction {
    fn simplify(&self) -> SimpleInstr<'_> {
        match self {
            tacky::Instruction::Label(label) => SimpleInstr::Label(label),
            tacky::Instruction::Jump { target } => SimpleInstr::UnconditionalJump(target),
            tacky::Instruction::JumpIfZero { target, .. }
            | tacky::Instruction::JumpIfNotZero { target, .. } => {
                SimpleInstr::ConditionalJump(target)
            }
            tacky::Instruction::Return(_) => SimpleInstr::Return,
            tacky::Instruction::SignExtend { .. }
            | tacky::Instruction::ZeroExtend { .. }
            | tacky::Instruction::Truncate { .. }
            | tacky::Instruction::IntToDouble { .. }
            | tacky::Instruction::DoubleToInt { .. }
            | tacky::Instruction::UIntToDouble { .. }
            | tacky::Instruction::DoubleToUInt { .. }
            | tacky::Instruction::Add { .. }
            | tacky::Instruction::Sub { .. }
            | tacky::Instruction::Mul { .. }
            | tacky::Instruction::DivSigned { .. }
            | tacky::Instruction::RemSigned { .. }
            | tacky::Instruction::BitAnd { .. }
            | tacky::Instruction::BitOr { .. }
            | tacky::Instruction::BitXor { .. }
            | tacky::Instruction::BitShiftLeft { .. }
            | tacky::Instruction::BitShiftRight { .. }
            | tacky::Instruction::Negate { .. }
            | tacky::Instruction::Complement { .. }
            | tacky::Instruction::Not { .. }
            | tacky::Instruction::Cmp { .. }
            | tacky::Instruction::Copy { .. }
            | tacky::Instruction::Load { .. }
            | tacky::Instruction::Store { .. }
            | tacky::Instruction::CopyBytes { .. }
            | tacky::Instruction::GetAddress { .. }
            | tacky::Instruction::AddPtr { .. }
            | tacky::Instruction::Call { .. } => SimpleInstr::Other,
        }
    }
}

impl CfgInstruction for assembly::Instr {
    fn simplify(&self) -> SimpleInstr<'_> {
        match self {
            assembly::Instr::Label(label) => SimpleInstr::Label(label),
            assembly::Instr::Jmp(target) => SimpleInstr::UnconditionalJump(target),
            assembly::Instr::JmpCC { label, .. } => SimpleInstr::ConditionalJump(label),
            assembly::Instr::Ret => SimpleInstr::Return,
            assembly::Instr::Mov { .. }
            | assembly::Instr::Movq { .. }
            | assembly::Instr::MovByte { .. }
            | assembly::Instr::Movabsq { .. }
            | assembly::Instr::Movsx { .. }
            | assembly::Instr::MovZeroExtend { .. }
            | assembly::Instr::MovSignExtendByte { .. }
            | assembly::Instr::Movsd { .. }
            | assembly::Instr::MovsdLoad { .. }
            | assembly::Instr::Lea { .. }
            | assembly::Instr::Cmp { .. }
            | assembly::Instr::Cmpq { .. }
            | assembly::Instr::CmpDouble { .. }
            | assembly::Instr::BinaryOp { .. }
            | assembly::Instr::Idiv(_)
            | assembly::Instr::Div(_)
            | assembly::Instr::Idivq(_)
            | assembly::Instr::Divq(_)
            | assembly::Instr::Cdq
            | assembly::Instr::Cqo
            | assembly::Instr::Cltq
            | assembly::Instr::Cvtsi2sd { .. }
            | assembly::Instr::Cvttsd2si { .. }
            | assembly::Instr::Unary { .. }
            | assembly::Instr::UnaryQ { .. }
            | assembly::Instr::Call(_)
            | assembly::Instr::Push(_)
            | assembly::Instr::Pop(_)
            | assembly::Instr::SetCC { .. }
            | assembly::Instr::AllocateStack(_)
            | assembly::Instr::DeallocateStack(_)
            | assembly::Instr::Comment(_) => SimpleInstr::Other,
        }
    }
}
