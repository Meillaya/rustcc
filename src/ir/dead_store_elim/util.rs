use crate::ir::tacky::{Instruction, Val};

use super::liveness::add_val;
use super::{AddressFacts, LiveSet};

pub(super) fn add_known_memory_uses(
    live_vars: &mut LiveSet,
    instruction: &Instruction,
    address_of: &AddressFacts,
) {
    match instruction {
        Instruction::Load { src_pointer, .. } => add_known_base(live_vars, src_pointer, address_of),
        Instruction::Store { dst_pointer, .. } => {
            add_known_base(live_vars, dst_pointer, address_of);
        }
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            ..
        } => {
            add_known_base(live_vars, src_pointer, address_of);
            add_known_base(live_vars, dst_pointer, address_of);
        }
        _ => {}
    }
}

fn add_known_base(live_vars: &mut LiveSet, val: &Val, address_of: &AddressFacts) {
    if let Some(base) = pointer_base(val, address_of) {
        live_vars.insert(base.to_string());
    }
}

pub(super) fn add_instruction_uses(live_vars: &mut LiveSet, instruction: &Instruction) {
    match instruction {
        Instruction::Return(val) => add_val(live_vars, val),
        Instruction::Copy { src, .. }
        | Instruction::SignExtend { src, .. }
        | Instruction::ZeroExtend { src, .. }
        | Instruction::Truncate { src, .. }
        | Instruction::IntToDouble { src, .. }
        | Instruction::DoubleToInt { src, .. }
        | Instruction::UIntToDouble { src, .. }
        | Instruction::DoubleToUInt { src, .. } => add_val(live_vars, src),
        Instruction::Add { src, dst }
        | Instruction::Sub { src, dst }
        | Instruction::Mul { src, dst }
        | Instruction::DivSigned { src, dst }
        | Instruction::RemSigned { src, dst }
        | Instruction::BitAnd { src, dst }
        | Instruction::BitOr { src, dst }
        | Instruction::BitXor { src, dst }
        | Instruction::BitShiftLeft { src, dst }
        | Instruction::BitShiftRight { src, dst } => {
            add_val(live_vars, src);
            live_vars.insert(dst.clone());
        }
        Instruction::Cmp { left, right, .. } => {
            add_val(live_vars, left);
            add_val(live_vars, right);
        }
        Instruction::JumpIfZero { condition, .. }
        | Instruction::JumpIfNotZero { condition, .. } => add_val(live_vars, condition),
        Instruction::Load { src_pointer, .. } => add_val(live_vars, src_pointer),
        Instruction::Store { src, dst_pointer } => {
            add_val(live_vars, src);
            add_val(live_vars, dst_pointer);
        }
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            ..
        } => {
            add_val(live_vars, src_pointer);
            add_val(live_vars, dst_pointer);
        }
        Instruction::AddPtr { ptr, index, .. } => {
            add_val(live_vars, ptr);
            add_val(live_vars, index);
        }
        Instruction::Call { args, .. } => {
            for arg in args {
                add_val(live_vars, arg);
            }
        }
        Instruction::Negate { dst }
        | Instruction::Complement { dst }
        | Instruction::Not { dst } => {
            live_vars.insert(dst.clone());
        }
        Instruction::Jump { .. } | Instruction::Label(_) | Instruction::GetAddress { .. } => {}
    }
}

// Mirrors nqcc2/lib/optimizations/optimize_utils.ml:3-21.
pub(super) fn instruction_dst(instruction: &Instruction) -> Option<String> {
    match instruction {
        Instruction::Copy { dst, .. }
        | Instruction::SignExtend { dst, .. }
        | Instruction::ZeroExtend { dst, .. }
        | Instruction::Truncate { dst, .. }
        | Instruction::IntToDouble { dst, .. }
        | Instruction::DoubleToInt { dst, .. }
        | Instruction::UIntToDouble { dst, .. }
        | Instruction::DoubleToUInt { dst, .. }
        | Instruction::Load { dst, .. }
        | Instruction::GetAddress { dst, .. }
        | Instruction::AddPtr { dst, .. }
        | Instruction::Cmp { dst, .. } => Some(dst.clone()),
        Instruction::Call { dst, .. } => dst.clone(),
        Instruction::Add { dst, .. }
        | Instruction::Sub { dst, .. }
        | Instruction::Mul { dst, .. }
        | Instruction::DivSigned { dst, .. }
        | Instruction::RemSigned { dst, .. }
        | Instruction::BitAnd { dst, .. }
        | Instruction::BitOr { dst, .. }
        | Instruction::BitXor { dst, .. }
        | Instruction::BitShiftLeft { dst, .. }
        | Instruction::BitShiftRight { dst, .. }
        | Instruction::Negate { dst }
        | Instruction::Complement { dst }
        | Instruction::Not { dst } => Some(dst.clone()),
        Instruction::Return(_)
        | Instruction::Jump { .. }
        | Instruction::JumpIfZero { .. }
        | Instruction::JumpIfNotZero { .. }
        | Instruction::Label(_)
        | Instruction::Store { .. }
        | Instruction::CopyBytes { .. } => None,
    }
}

pub(super) fn pointer_base<'a>(val: &'a Val, address_of: &'a AddressFacts) -> Option<&'a str> {
    match val {
        Val::Var(name) => address_of.get(name).map(String::as_str),
        Val::Constant(_) | Val::ConstantDouble(_) => None,
    }
}

pub(super) fn update_address_facts(address_of: &mut AddressFacts, instruction: &Instruction) {
    if let Some(dst) = instruction_dst(instruction) {
        address_of.remove(&dst);
    }
    match instruction {
        Instruction::GetAddress { src, dst } => {
            address_of.insert(dst.clone(), src.clone());
        }
        Instruction::AddPtr { ptr, dst, .. } => {
            if let Some(base) = pointer_base(ptr, address_of).map(str::to_string) {
                address_of.insert(dst.clone(), base);
            }
        }
        _ => {}
    }
}
