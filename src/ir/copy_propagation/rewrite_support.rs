use std::collections::BTreeSet;

use crate::ir::cfg::BasicBlock;
use crate::ir::tacky::{Instruction, OperandType, TypeEnv, Val};

use super::facts::{ReachingCopies, ValKey, var_type};

pub(super) fn collect_write_pointers(
    blocks: &[BasicBlock<ReachingCopies, Instruction>],
) -> BTreeSet<String> {
    let mut write_pointers = BTreeSet::new();
    let mut pointer_sources = Vec::<(String, String)>::new();
    for block in blocks {
        for (_, instruction) in &block.instructions {
            match instruction {
                Instruction::Store { dst_pointer, .. }
                | Instruction::CopyBytes { dst_pointer, .. } => {
                    if let Val::Var(name) = dst_pointer {
                        write_pointers.insert(name.clone());
                    }
                }
                Instruction::Call { args, .. } => {
                    for arg in args {
                        if let Val::Var(name) = arg {
                            write_pointers.insert(name.clone());
                        }
                    }
                }
                Instruction::Copy {
                    src: Val::Var(src),
                    dst,
                }
                | Instruction::AddPtr {
                    ptr: Val::Var(src),
                    dst,
                    ..
                } => pointer_sources.push((dst.clone(), src.clone())),
                _ => {}
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for (dst, src) in &pointer_sources {
            if write_pointers.contains(dst) && write_pointers.insert(src.clone()) {
                changed = true;
            }
        }
    }
    write_pointers
}

pub(super) fn replace_instruction_sources(
    instruction: &Instruction,
    copies: &ReachingCopies,
    type_env: &TypeEnv,
    write_pointers: &BTreeSet<String>,
) -> Instruction {
    match instruction {
        Instruction::Return(val) => Instruction::Return(replace_val(val, copies)),
        Instruction::Copy { src, dst } => Instruction::Copy {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::SignExtend { src, dst } => Instruction::SignExtend {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::ZeroExtend { src, dst } => Instruction::ZeroExtend {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::Truncate { src, dst } => Instruction::Truncate {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::IntToDouble { src, dst } => Instruction::IntToDouble {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::DoubleToInt { src, dst } => Instruction::DoubleToInt {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::UIntToDouble { src, dst } => Instruction::UIntToDouble {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::DoubleToUInt { src, dst } => Instruction::DoubleToUInt {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::Add { src, dst } => Instruction::Add {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::Sub { src, dst } => Instruction::Sub {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::Mul { src, dst } => Instruction::Mul {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::DivSigned { src, dst } => Instruction::DivSigned {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::RemSigned { src, dst } => Instruction::RemSigned {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::BitAnd { src, dst } => Instruction::BitAnd {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::BitOr { src, dst } => Instruction::BitOr {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::BitXor { src, dst } => Instruction::BitXor {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::BitShiftLeft { src, dst } => Instruction::BitShiftLeft {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::BitShiftRight { src, dst } => Instruction::BitShiftRight {
            src: replace_val(src, copies),
            dst: dst.clone(),
        },
        Instruction::Cmp {
            left,
            right,
            dst,
            cc,
        } => Instruction::Cmp {
            left: replace_val(left, copies),
            right: replace_val(right, copies),
            dst: dst.clone(),
            cc: *cc,
        },
        Instruction::JumpIfZero { condition, target } => Instruction::JumpIfZero {
            condition: replace_val(condition, copies),
            target: target.clone(),
        },
        Instruction::JumpIfNotZero { condition, target } => Instruction::JumpIfNotZero {
            condition: replace_val(condition, copies),
            target: target.clone(),
        },
        Instruction::Load { src_pointer, dst } => Instruction::Load {
            src_pointer: replace_val(src_pointer, copies),
            dst: dst.clone(),
        },
        Instruction::Store { src, dst_pointer } => Instruction::Store {
            src: replace_val(src, copies),
            dst_pointer: replace_val(dst_pointer, copies),
        },
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            size,
        } => Instruction::CopyBytes {
            src_pointer: replace_val(src_pointer, copies),
            dst_pointer: replace_val(dst_pointer, copies),
            size: *size,
        },
        Instruction::AddPtr {
            ptr,
            index,
            scale,
            dst,
        } => Instruction::AddPtr {
            ptr: replace_val(ptr, copies),
            index: replace_val(index, copies),
            scale: *scale,
            dst: dst.clone(),
        },
        Instruction::Call { name, args, dst } => Instruction::Call {
            name: name.clone(),
            args: args.iter().map(|arg| replace_val(arg, copies)).collect(),
            dst: dst.clone(),
        },
        Instruction::GetAddress { src, dst } => Instruction::GetAddress {
            src: if write_pointers.contains(dst) {
                src.clone()
            } else {
                replace_address_source(src, copies, type_env)
            },
            dst: dst.clone(),
        },
        Instruction::Negate { .. }
        | Instruction::Complement { .. }
        | Instruction::Not { .. }
        | Instruction::Jump { .. }
        | Instruction::Label(_) => instruction.clone(),
    }
}

fn replace_val(val: &Val, copies: &ReachingCopies) -> Val {
    let Val::Var(_) = val else {
        return val.clone();
    };
    let key = ValKey::from_val(val);
    copies
        .iter()
        .find(|copy| copy.dst == key)
        .map_or_else(|| val.clone(), |copy| copy.src.to_val())
}

fn replace_address_source(src: &str, copies: &ReachingCopies, type_env: &TypeEnv) -> String {
    let key = ValKey::Var(src.to_string());
    copies
        .iter()
        .find(|copy| copy.dst == key)
        .and_then(|copy| match &copy.src {
            ValKey::Var(name)
                if matches!(
                    (var_type(name, type_env), var_type(src, type_env)),
                    (OperandType::ByteArray { .. }, OperandType::ByteArray { .. })
                ) =>
            {
                Some(name.clone())
            }
            ValKey::Var(_) | ValKey::Constant(_) | ValKey::ConstantDouble(_) => None,
        })
        .unwrap_or_else(|| src.to_string())
}
