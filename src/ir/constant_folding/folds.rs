//! Operation-specific folds for constant folding.
//!
//! Mirrors nqcc2/lib/optimizations/constant_folding.ml:132-153.

use std::collections::HashMap;

use crate::ir::const_eval::{
    BinaryOp, ConstVal, UnaryOp, evaluate_binary, evaluate_cast, evaluate_unary,
};
use crate::ir::constant_folding::util::{const_for_val, same_val, value_type, var_type};
use crate::ir::tacky::{Instruction, TypeEnv, Val};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CastOp {
    SignExtend,
    ZeroExtend,
    Truncate,
    IntToDouble,
    DoubleToInt,
    UIntToDouble,
    DoubleToUInt,
}

pub(super) fn fold_copy(
    src: Val,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> (Instruction, bool) {
    let dst_ty = var_type(&dst, type_env);
    match const_for_val(&src, dst_ty, constants).map(|value| evaluate_cast(value, dst_ty)) {
        Some(value) => {
            let folded_src = value.to_val();
            let changed = !same_val(&src, &folded_src);
            constants.insert(dst.clone(), value);
            (
                Instruction::Copy {
                    src: folded_src,
                    dst,
                },
                changed,
            )
        }
        None => {
            constants.remove(&dst);
            (Instruction::Copy { src, dst }, false)
        }
    }
}

pub(super) fn fold_cast(
    op: CastOp,
    src: Val,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> (Instruction, bool) {
    let dst_ty = var_type(&dst, type_env);
    match const_for_val(&src, value_type(&src, type_env, constants), constants)
        .map(|value| evaluate_cast(value, dst_ty))
    {
        Some(value) => {
            constants.insert(dst.clone(), value);
            (
                Instruction::Copy {
                    src: value.to_val(),
                    dst,
                },
                true,
            )
        }
        None => {
            constants.remove(&dst);
            (cast_instruction(op, src, dst), false)
        }
    }
}

pub(super) fn fold_unary(
    op: UnaryOp,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> (Instruction, bool) {
    match constants
        .get(&dst)
        .copied()
        .and_then(|value| evaluate_unary(op, value))
        .map(|value| evaluate_cast(value, var_type(&dst, type_env)))
    {
        Some(value) => {
            constants.insert(dst.clone(), value);
            (
                Instruction::Copy {
                    src: value.to_val(),
                    dst,
                },
                true,
            )
        }
        None => {
            constants.remove(&dst);
            (unary_instruction(op, dst), false)
        }
    }
}

pub(super) fn fold_binary(
    op: BinaryOp,
    src: Val,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> (Instruction, bool) {
    let dst_ty = var_type(&dst, type_env);
    let folded = constants
        .get(&dst)
        .copied()
        .zip(const_for_val(&src, dst_ty, constants))
        .and_then(|(left, right)| evaluate_binary(op, left, right))
        .map(|value| evaluate_cast(value, dst_ty));
    match folded {
        Some(value) => {
            constants.insert(dst.clone(), value);
            (
                Instruction::Copy {
                    src: value.to_val(),
                    dst,
                },
                true,
            )
        }
        None => {
            constants.remove(&dst);
            (binary_instruction(op, src, dst), false)
        }
    }
}

fn cast_instruction(op: CastOp, src: Val, dst: String) -> Instruction {
    match op {
        CastOp::SignExtend => Instruction::SignExtend { src, dst },
        CastOp::ZeroExtend => Instruction::ZeroExtend { src, dst },
        CastOp::Truncate => Instruction::Truncate { src, dst },
        CastOp::IntToDouble => Instruction::IntToDouble { src, dst },
        CastOp::DoubleToInt => Instruction::DoubleToInt { src, dst },
        CastOp::UIntToDouble => Instruction::UIntToDouble { src, dst },
        CastOp::DoubleToUInt => Instruction::DoubleToUInt { src, dst },
    }
}

fn unary_instruction(op: UnaryOp, dst: String) -> Instruction {
    match op {
        UnaryOp::Negate => Instruction::Negate { dst },
        UnaryOp::Complement => Instruction::Complement { dst },
        UnaryOp::Not => Instruction::Not { dst },
    }
}

fn binary_instruction(op: BinaryOp, src: Val, dst: String) -> Instruction {
    match op {
        BinaryOp::Add => Instruction::Add { src, dst },
        BinaryOp::Subtract => Instruction::Sub { src, dst },
        BinaryOp::Multiply => Instruction::Mul { src, dst },
        BinaryOp::Divide => Instruction::DivSigned { src, dst },
        BinaryOp::Remainder => Instruction::RemSigned { src, dst },
        BinaryOp::BitAnd => Instruction::BitAnd { src, dst },
        BinaryOp::BitOr => Instruction::BitOr { src, dst },
        BinaryOp::BitXor => Instruction::BitXor { src, dst },
        BinaryOp::ShiftLeft => Instruction::BitShiftLeft { src, dst },
        BinaryOp::ShiftRight => Instruction::BitShiftRight { src, dst },
    }
}
