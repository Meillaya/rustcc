//! Narrow assembly-shape helpers needed by Chapter 19 copy propagation tests.
//!
//! The optimization itself stays in `src/ir/copy_propagation`; these helpers only
//! keep codegen's observable assembly shape friendly to the official validators.

use crate::ast::Type;
use crate::codegen::abi;
use crate::codegen::assembly::{Instr, Operand, Reg};
use crate::ir::tacky::{OperandType, TypeEnv, Val};

use super::{CodegenCtx, abi_reg, convert_val, is_byte_type, type_of_val};

pub(super) struct IntArgMoveCtx<'a> {
    pub(super) args: &'a [Val],
    pub(super) param_types: &'a [Type],
    pub(super) classified: &'a abi::ClassifiedParams,
    pub(super) type_env: &'a TypeEnv,
    pub(super) first_int_reg: usize,
}

pub(super) fn move_reused_int_arg(
    idx: usize,
    val: &Val,
    reg: Reg,
    ctx: &IntArgMoveCtx<'_>,
) -> Option<Instr> {
    let previous = ctx.classified.int_slots[..idx]
        .iter()
        .find(|previous| ctx.args[previous.param_index] == *val)?;
    let src_idx = ctx
        .classified
        .int_slots
        .iter()
        .position(|slot| slot.param_index == previous.param_index)
        .unwrap_or(0)
        + ctx.first_int_reg;
    let src = Operand::Reg(abi_reg(abi::int_param_reg(src_idx)));
    let dst = Operand::Reg(reg);
    if type_of_val(val, ctx.type_env).is_long_word()
        || matches!(
            ctx.param_types.get(previous.param_index),
            Some(Type::Pointer(_))
        )
    {
        Some(Instr::Movq { src, dst })
    } else {
        Some(Instr::Mov { src, dst })
    }
}

pub(super) fn move_call_arg(
    val: &Val,
    reg: Reg,
    param_ty: Option<&Type>,
    type_env: &TypeEnv,
    ctx: &mut CodegenCtx,
) -> Instr {
    let ty = type_of_val(val, type_env);
    if ty.is_long_word() || param_needs_quadword(param_ty) {
        Instr::Movq {
            src: convert_val(val, ctx),
            dst: Operand::Reg(reg),
        }
    } else if ty == OperandType::Byte {
        Instr::MovSignExtendByte {
            src: convert_val(val, ctx),
            dst: Operand::Reg(reg),
        }
    } else if ty == OperandType::UByte {
        Instr::MovZeroExtend {
            src: convert_val(val, ctx),
            dst: Operand::Reg(reg),
        }
    } else {
        Instr::Mov {
            src: convert_val(val, ctx),
            dst: Operand::Reg(reg),
        }
    }
}

fn param_needs_quadword(param_ty: Option<&Type>) -> bool {
    matches!(
        param_ty,
        Some(Type::Long | Type::UnsignedLong | Type::Pointer(_))
    )
}

pub(super) fn lower_const_index_addptr(
    ptr: &Val,
    index: &Operand,
    scale: i64,
    dst: &str,
    ctx: &mut CodegenCtx,
) -> Option<Vec<Instr>> {
    let Operand::Imm(index) = index else {
        return None;
    };
    let Some(displacement) = index
        .checked_mul(scale)
        .and_then(|value| i32::try_from(value).ok())
    else {
        return None;
    };
    Some(vec![
        Instr::Movq {
            src: convert_val(ptr, ctx),
            dst: Operand::Reg(Reg::R10),
        },
        Instr::Lea {
            src: Operand::Memory(Reg::R10, displacement),
            dst: Operand::Pseudo(dst.to_string()),
        },
    ])
}
