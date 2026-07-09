//! Instruction-level constant folding.
//!
//! Mirrors nqcc2/lib/optimizations/constant_folding.ml:132-155.

use std::collections::HashMap;

use crate::ir::const_eval::{BinaryOp, ConstVal, UnaryOp, evaluate_cmp};
use crate::ir::constant_folding::folds::{CastOp, fold_binary, fold_cast, fold_copy, fold_unary};
use crate::ir::constant_folding::util::{
    comparison_type, const_for_val, value_is_zero, value_type,
};
use crate::ir::tacky::{Instruction, TypeEnv};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InstructionResult {
    pub(super) instruction: Option<Instruction>,
    pub(super) changed: bool,
}

impl InstructionResult {
    fn keep(instruction: Instruction) -> Self {
        Self {
            instruction: Some(instruction),
            changed: false,
        }
    }

    fn replace(instruction: Instruction, changed: bool) -> Self {
        Self {
            instruction: Some(instruction),
            changed,
        }
    }

    fn remove() -> Self {
        Self {
            instruction: None,
            changed: true,
        }
    }
}

pub(super) fn optimize_instruction(
    instruction: Instruction,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> InstructionResult {
    match instruction {
        Instruction::Copy { src, dst } => {
            let (instruction, changed) = fold_copy(src, dst, type_env, constants);
            InstructionResult::replace(instruction, changed)
        }
        Instruction::SignExtend { src, dst } => {
            fold_cast_result(CastOp::SignExtend, src, dst, type_env, constants)
        }
        Instruction::ZeroExtend { src, dst } => {
            fold_cast_result(CastOp::ZeroExtend, src, dst, type_env, constants)
        }
        Instruction::Truncate { src, dst } => {
            fold_cast_result(CastOp::Truncate, src, dst, type_env, constants)
        }
        Instruction::IntToDouble { src, dst } => {
            fold_cast_result(CastOp::IntToDouble, src, dst, type_env, constants)
        }
        Instruction::DoubleToInt { src, dst } => {
            fold_cast_result(CastOp::DoubleToInt, src, dst, type_env, constants)
        }
        Instruction::UIntToDouble { src, dst } => {
            fold_cast_result(CastOp::UIntToDouble, src, dst, type_env, constants)
        }
        Instruction::DoubleToUInt { src, dst } => {
            fold_cast_result(CastOp::DoubleToUInt, src, dst, type_env, constants)
        }
        Instruction::Negate { dst } => fold_unary_result(UnaryOp::Negate, dst, type_env, constants),
        Instruction::Complement { dst } => {
            fold_unary_result(UnaryOp::Complement, dst, type_env, constants)
        }
        Instruction::Not { dst } => fold_unary_result(UnaryOp::Not, dst, type_env, constants),
        Instruction::Add { src, dst } => {
            fold_binary_result(BinaryOp::Add, src, dst, type_env, constants)
        }
        Instruction::Sub { src, dst } => {
            fold_binary_result(BinaryOp::Subtract, src, dst, type_env, constants)
        }
        Instruction::Mul { src, dst } => {
            fold_binary_result(BinaryOp::Multiply, src, dst, type_env, constants)
        }
        Instruction::DivSigned { src, dst } => {
            fold_binary_result(BinaryOp::Divide, src, dst, type_env, constants)
        }
        Instruction::RemSigned { src, dst } => {
            fold_binary_result(BinaryOp::Remainder, src, dst, type_env, constants)
        }
        Instruction::BitAnd { src, dst } => {
            fold_binary_result(BinaryOp::BitAnd, src, dst, type_env, constants)
        }
        Instruction::BitOr { src, dst } => {
            fold_binary_result(BinaryOp::BitOr, src, dst, type_env, constants)
        }
        Instruction::BitXor { src, dst } => {
            fold_binary_result(BinaryOp::BitXor, src, dst, type_env, constants)
        }
        Instruction::BitShiftLeft { src, dst } => {
            fold_binary_result(BinaryOp::ShiftLeft, src, dst, type_env, constants)
        }
        Instruction::BitShiftRight { src, dst } => {
            fold_binary_result(BinaryOp::ShiftRight, src, dst, type_env, constants)
        }
        Instruction::Cmp {
            left,
            right,
            dst,
            cc,
        } => {
            let cmp_ty = comparison_type(&left, &right, type_env, constants);
            let folded = const_for_val(&left, cmp_ty, constants)
                .zip(const_for_val(&right, cmp_ty, constants))
                .and_then(|(left, right)| evaluate_cmp(cc, left, right));
            match folded {
                Some(value) => {
                    constants.insert(dst.clone(), value);
                    InstructionResult::replace(
                        Instruction::Copy {
                            src: value.to_val(),
                            dst,
                        },
                        true,
                    )
                }
                None => {
                    constants.remove(&dst);
                    InstructionResult::keep(Instruction::Cmp {
                        left,
                        right,
                        dst,
                        cc,
                    })
                }
            }
        }
        Instruction::JumpIfZero { condition, target } => match const_for_val(
            &condition,
            value_type(&condition, type_env, constants),
            constants,
        ) {
            Some(value) if value_is_zero(value) => {
                InstructionResult::replace(Instruction::Jump { target }, true)
            }
            Some(_) => InstructionResult::remove(),
            None => InstructionResult::keep(Instruction::JumpIfZero { condition, target }),
        },
        Instruction::JumpIfNotZero { condition, target } => match const_for_val(
            &condition,
            value_type(&condition, type_env, constants),
            constants,
        ) {
            Some(value) if value_is_zero(value) => InstructionResult::remove(),
            Some(_) => InstructionResult::replace(Instruction::Jump { target }, true),
            None => InstructionResult::keep(Instruction::JumpIfNotZero { condition, target }),
        },
        Instruction::Load { dst, src_pointer } => {
            constants.remove(&dst);
            InstructionResult::keep(Instruction::Load { src_pointer, dst })
        }
        Instruction::Call { name, args, dst } => {
            constants.clear();
            InstructionResult::keep(Instruction::Call { name, args, dst })
        }
        Instruction::Store { src, dst_pointer } => {
            constants.clear();
            InstructionResult::keep(Instruction::Store { src, dst_pointer })
        }
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            size,
        } => {
            constants.clear();
            InstructionResult::keep(Instruction::CopyBytes {
                src_pointer,
                dst_pointer,
                size,
            })
        }
        Instruction::GetAddress { src, dst } => {
            constants.remove(&dst);
            InstructionResult::keep(Instruction::GetAddress { src, dst })
        }
        Instruction::AddPtr {
            ptr,
            index,
            scale,
            dst,
        } => {
            constants.remove(&dst);
            InstructionResult::keep(Instruction::AddPtr {
                ptr,
                index,
                scale,
                dst,
            })
        }
        Instruction::Return(_) | Instruction::Jump { .. } | Instruction::Label(_) => {
            InstructionResult::keep(instruction)
        }
    }
}

fn fold_cast_result(
    op: CastOp,
    src: crate::ir::tacky::Val,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> InstructionResult {
    let (instruction, changed) = fold_cast(op, src, dst, type_env, constants);
    InstructionResult::replace(instruction, changed)
}

fn fold_unary_result(
    op: UnaryOp,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> InstructionResult {
    let (instruction, changed) = fold_unary(op, dst, type_env, constants);
    InstructionResult::replace(instruction, changed)
}

fn fold_binary_result(
    op: BinaryOp,
    src: crate::ir::tacky::Val,
    dst: String,
    type_env: &TypeEnv,
    constants: &mut HashMap<String, ConstVal>,
) -> InstructionResult {
    let (instruction, changed) = fold_binary(op, src, dst, type_env, constants);
    InstructionResult::replace(instruction, changed)
}
