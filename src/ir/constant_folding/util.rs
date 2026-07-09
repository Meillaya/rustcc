//! Shared helpers for the TACKY constant-folding pass.

use std::collections::HashMap;

use crate::ir::const_eval::{ConstVal, UnaryOp, evaluate_unary, val_to_const};
use crate::ir::tacky::{OperandType, TypeEnv, Val};

pub(super) fn const_for_val(
    val: &Val,
    ty: OperandType,
    constants: &HashMap<String, ConstVal>,
) -> Option<ConstVal> {
    match val {
        Val::Var(name) => constants.get(name).copied(),
        Val::Constant(_) | Val::ConstantDouble(_) => val_to_const(val, ty),
    }
}

pub(super) fn var_type(name: &str, type_env: &TypeEnv) -> OperandType {
    type_env.get(name).copied().unwrap_or(OperandType::Int)
}

pub(super) fn value_type(
    val: &Val,
    type_env: &TypeEnv,
    constants: &HashMap<String, ConstVal>,
) -> OperandType {
    match val {
        Val::Var(name) => type_env.get(name).copied().unwrap_or(OperandType::Int),
        Val::ConstantDouble(_) => OperandType::Double,
        Val::Constant(_) => match const_for_val(val, OperandType::Int, constants) {
            Some(ConstVal::Long(_)) => OperandType::Long,
            Some(ConstVal::ULong(_)) => OperandType::ULong,
            Some(ConstVal::UInt(_)) => OperandType::UInt,
            Some(ConstVal::Double(_)) => OperandType::Double,
            Some(ConstVal::Int(_)) | None => OperandType::Int,
        },
    }
}

pub(super) fn comparison_type(
    left: &Val,
    right: &Val,
    type_env: &TypeEnv,
    constants: &HashMap<String, ConstVal>,
) -> OperandType {
    let left_ty = value_type(left, type_env, constants);
    let right_ty = value_type(right, type_env, constants);
    if left_ty == OperandType::Double || right_ty == OperandType::Double {
        OperandType::Double
    } else if left_ty == OperandType::ULong || right_ty == OperandType::ULong {
        OperandType::ULong
    } else if left_ty == OperandType::Long || right_ty == OperandType::Long {
        if left_ty.is_unsigned() || right_ty.is_unsigned() {
            OperandType::ULong
        } else {
            OperandType::Long
        }
    } else if left_ty.is_unsigned() || right_ty.is_unsigned() {
        OperandType::UInt
    } else {
        OperandType::Int
    }
}

pub(super) fn value_is_zero(value: ConstVal) -> bool {
    matches!(evaluate_unary(UnaryOp::Not, value), Some(ConstVal::Int(1)))
}

pub(super) fn same_val(left: &Val, right: &Val) -> bool {
    match (left, right) {
        (Val::Constant(left), Val::Constant(right)) => left == right,
        (Val::Var(left), Val::Var(right)) => left == right,
        (Val::ConstantDouble(left), Val::ConstantDouble(right)) => {
            left.to_bits() == right.to_bits()
        }
        _ => false,
    }
}
