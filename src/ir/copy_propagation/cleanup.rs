use std::collections::{BTreeMap, BTreeSet};

use crate::ir::tacky::{Instruction, Val};

use super::facts::pointer_base;

pub(super) fn cleanup_unused_aggregate_scaffolding(
    mut instructions: Vec<Instruction>,
) -> Vec<Instruction> {
    loop {
        let used_vars = used_value_vars(&instructions);
        let address_of = address_facts(&instructions);
        let before_len = instructions.len();
        instructions.retain(|instruction| match instruction {
            Instruction::GetAddress { dst, .. } => used_vars.contains(dst),
            Instruction::CopyBytes { dst_pointer, .. } => {
                !copybytes_writes_unused_temp(dst_pointer, &address_of, &used_vars)
            }
            _ => true,
        });
        if instructions.len() == before_len {
            return instructions;
        }
    }
}

fn used_value_vars(instructions: &[Instruction]) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for instruction in instructions {
        collect_instruction_uses(instruction, &mut used);
    }
    used
}

fn address_facts(instructions: &[Instruction]) -> BTreeMap<String, String> {
    instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::GetAddress { src, dst } => Some((dst.clone(), src.clone())),
            _ => None,
        })
        .collect()
}

fn copybytes_writes_unused_temp(
    dst_pointer: &Val,
    address_of: &BTreeMap<String, String>,
    used_vars: &BTreeSet<String>,
) -> bool {
    let Some(base) = pointer_base(dst_pointer, address_of) else {
        return false;
    };
    base.starts_with("tmp.") && !used_vars.contains(base)
}

fn collect_instruction_uses(instruction: &Instruction, used: &mut BTreeSet<String>) {
    match instruction {
        Instruction::Return(val) => collect_val_use(val, used),
        Instruction::Copy { src, .. }
        | Instruction::SignExtend { src, .. }
        | Instruction::ZeroExtend { src, .. }
        | Instruction::Truncate { src, .. }
        | Instruction::IntToDouble { src, .. }
        | Instruction::DoubleToInt { src, .. }
        | Instruction::UIntToDouble { src, .. }
        | Instruction::DoubleToUInt { src, .. }
        | Instruction::Add { src, .. }
        | Instruction::Sub { src, .. }
        | Instruction::Mul { src, .. }
        | Instruction::DivSigned { src, .. }
        | Instruction::RemSigned { src, .. }
        | Instruction::BitAnd { src, .. }
        | Instruction::BitOr { src, .. }
        | Instruction::BitXor { src, .. }
        | Instruction::BitShiftLeft { src, .. }
        | Instruction::BitShiftRight { src, .. } => collect_val_use(src, used),
        Instruction::Cmp { left, right, .. } => {
            collect_val_use(left, used);
            collect_val_use(right, used);
        }
        Instruction::JumpIfZero { condition, .. }
        | Instruction::JumpIfNotZero { condition, .. } => collect_val_use(condition, used),
        Instruction::Load { src_pointer, .. } => collect_val_use(src_pointer, used),
        Instruction::Store { src, dst_pointer } => {
            collect_val_use(src, used);
            collect_val_use(dst_pointer, used);
        }
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            ..
        } => {
            collect_val_use(src_pointer, used);
            collect_val_use(dst_pointer, used);
        }
        Instruction::AddPtr { ptr, index, .. } => {
            collect_val_use(ptr, used);
            collect_val_use(index, used);
        }
        Instruction::Call { args, .. } => {
            for arg in args {
                collect_val_use(arg, used);
            }
        }
        Instruction::Negate { .. }
        | Instruction::Complement { .. }
        | Instruction::Not { .. }
        | Instruction::Jump { .. }
        | Instruction::Label(_)
        | Instruction::GetAddress { .. } => {}
    }
}

fn collect_val_use(val: &Val, used: &mut BTreeSet<String>) {
    if let Val::Var(name) = val {
        used.insert(name.clone());
    }
}
