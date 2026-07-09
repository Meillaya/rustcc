//! Constant evaluation for TACKY optimization.
//!
//! Mirrors nqcc2/lib/optimizations/constant_folding.ml:3-155.  The Rust
//! TACKY IR uses two-address arithmetic instructions (`Copy lhs; Add rhs`), so
//! this module exposes typed helpers that the pass can apply to the current
//! constant value of the destination and the constant source operand.

use crate::ir::tacky::{ConditionCode, OperandType, Val};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstVal {
    Int(i32),
    UInt(u32),
    Long(i64),
    ULong(u64),
    Double(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Complement,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

impl ConstVal {
    pub fn to_val(self) -> Val {
        match self {
            ConstVal::Int(value) => Val::Constant(i64::from(value)),
            ConstVal::UInt(value) => Val::Constant(i64::from(value)),
            ConstVal::Long(value) => Val::Constant(value),
            ConstVal::ULong(value) => Val::Constant(i64::from_ne_bytes(value.to_ne_bytes())),
            ConstVal::Double(value) => Val::ConstantDouble(value),
        }
    }

    fn is_zero(self) -> bool {
        match self {
            ConstVal::Int(value) => value == 0,
            ConstVal::UInt(value) => value == 0,
            ConstVal::Long(value) => value == 0,
            ConstVal::ULong(value) => value == 0,
            ConstVal::Double(value) => value == 0.0,
        }
    }
}

// Mirrors nqcc2/lib/optimizations/constant_folding.ml:3-13.
pub fn evaluate_cast(src: ConstVal, dst_type: OperandType) -> ConstVal {
    match dst_type {
        OperandType::Int => ConstVal::Int(as_i64(src) as i32),
        OperandType::Byte => ConstVal::Int(i32::from(as_i64(src) as i8)),
        OperandType::UInt => ConstVal::UInt(as_i64(src) as u32),
        OperandType::UByte => ConstVal::UInt(u32::from(as_i64(src) as u8)),
        OperandType::Long => ConstVal::Long(as_i64(src)),
        OperandType::ULong | OperandType::ByteArray { .. } => ConstVal::ULong(as_u64(src)),
        OperandType::Double => ConstVal::Double(as_f64(src)),
    }
}

// Mirrors nqcc2/lib/optimizations/constant_folding.ml:15.
fn int_of_bool(value: bool) -> ConstVal {
    ConstVal::Int(i32::from(value))
}

// Mirrors nqcc2/lib/optimizations/constant_folding.ml:46-50.
pub fn evaluate_unary(op: UnaryOp, value: ConstVal) -> Option<ConstVal> {
    Some(match (op, value) {
        (UnaryOp::Not, value) => int_of_bool(value.is_zero()),
        (UnaryOp::Negate, ConstVal::Int(value)) => ConstVal::Int(value.wrapping_neg()),
        (UnaryOp::Negate, ConstVal::UInt(value)) => ConstVal::UInt(value.wrapping_neg()),
        (UnaryOp::Negate, ConstVal::Long(value)) => ConstVal::Long(value.wrapping_neg()),
        (UnaryOp::Negate, ConstVal::ULong(value)) => ConstVal::ULong(value.wrapping_neg()),
        (UnaryOp::Negate, ConstVal::Double(value)) => ConstVal::Double(-value),
        (UnaryOp::Complement, ConstVal::Int(value)) => ConstVal::Int(!value),
        (UnaryOp::Complement, ConstVal::UInt(value)) => ConstVal::UInt(!value),
        (UnaryOp::Complement, ConstVal::Long(value)) => ConstVal::Long(!value),
        (UnaryOp::Complement, ConstVal::ULong(value)) => ConstVal::ULong(!value),
        (UnaryOp::Complement, ConstVal::Double(_)) => return None,
    })
}

// Mirrors nqcc2/lib/optimizations/constant_folding.ml:51-69.
pub fn evaluate_binary(op: BinaryOp, left: ConstVal, right: ConstVal) -> Option<ConstVal> {
    match (left, right) {
        (ConstVal::Int(left), ConstVal::Int(right)) => eval_i32(op, left, right),
        (ConstVal::UInt(left), ConstVal::UInt(right)) => eval_u32(op, left, right),
        (ConstVal::Long(left), ConstVal::Long(right)) => eval_i64(op, left, right),
        (ConstVal::ULong(left), ConstVal::ULong(right)) => eval_u64(op, left, right),
        (ConstVal::Double(left), ConstVal::Double(right)) => eval_f64(op, left, right),
        _ => None,
    }
}

pub fn evaluate_cmp(cc: ConditionCode, left: ConstVal, right: ConstVal) -> Option<ConstVal> {
    Some(int_of_bool(match (left, right) {
        (ConstVal::Int(left), ConstVal::Int(right)) => compare_order(cc, left.cmp(&right)),
        (ConstVal::UInt(left), ConstVal::UInt(right)) => compare_order(cc, left.cmp(&right)),
        (ConstVal::Long(left), ConstVal::Long(right)) => compare_order(cc, left.cmp(&right)),
        (ConstVal::ULong(left), ConstVal::ULong(right)) => compare_order(cc, left.cmp(&right)),
        (ConstVal::Double(left), ConstVal::Double(right)) => compare_double(cc, left, right)?,
        _ => return None,
    }))
}

pub fn val_to_const(val: &Val, ty: OperandType) -> Option<ConstVal> {
    match val {
        Val::Constant(value) => Some(evaluate_cast(ConstVal::Long(*value), ty)),
        Val::ConstantDouble(value) => Some(evaluate_cast(ConstVal::Double(*value), ty)),
        Val::Var(_) => None,
    }
}

fn eval_i32(op: BinaryOp, left: i32, right: i32) -> Option<ConstVal> {
    Some(ConstVal::Int(match op {
        BinaryOp::Add => left.wrapping_add(right),
        BinaryOp::Subtract => left.wrapping_sub(right),
        BinaryOp::Multiply => left.wrapping_mul(right),
        BinaryOp::Divide => left.checked_div(right).unwrap_or(0),
        BinaryOp::Remainder => left.checked_rem(right).unwrap_or(0),
        BinaryOp::BitAnd => left & right,
        BinaryOp::BitOr => left | right,
        BinaryOp::BitXor => left ^ right,
        BinaryOp::ShiftLeft => left.checked_shl(u32::try_from(right).ok()?).unwrap_or(0),
        BinaryOp::ShiftRight => left.checked_shr(u32::try_from(right).ok()?).unwrap_or(0),
    }))
}

fn eval_u32(op: BinaryOp, left: u32, right: u32) -> Option<ConstVal> {
    Some(ConstVal::UInt(match op {
        BinaryOp::Add => left.wrapping_add(right),
        BinaryOp::Subtract => left.wrapping_sub(right),
        BinaryOp::Multiply => left.wrapping_mul(right),
        BinaryOp::Divide => left.checked_div(right).unwrap_or(0),
        BinaryOp::Remainder => left.checked_rem(right).unwrap_or(0),
        BinaryOp::BitAnd => left & right,
        BinaryOp::BitOr => left | right,
        BinaryOp::BitXor => left ^ right,
        BinaryOp::ShiftLeft => left.checked_shl(right).unwrap_or(0),
        BinaryOp::ShiftRight => left.checked_shr(right).unwrap_or(0),
    }))
}

fn eval_i64(op: BinaryOp, left: i64, right: i64) -> Option<ConstVal> {
    Some(ConstVal::Long(match op {
        BinaryOp::Add => left.wrapping_add(right),
        BinaryOp::Subtract => left.wrapping_sub(right),
        BinaryOp::Multiply => left.wrapping_mul(right),
        BinaryOp::Divide => left.checked_div(right).unwrap_or(0),
        BinaryOp::Remainder => left.checked_rem(right).unwrap_or(0),
        BinaryOp::BitAnd => left & right,
        BinaryOp::BitOr => left | right,
        BinaryOp::BitXor => left ^ right,
        BinaryOp::ShiftLeft => left.checked_shl(u32::try_from(right).ok()?).unwrap_or(0),
        BinaryOp::ShiftRight => left.checked_shr(u32::try_from(right).ok()?).unwrap_or(0),
    }))
}

fn eval_u64(op: BinaryOp, left: u64, right: u64) -> Option<ConstVal> {
    Some(ConstVal::ULong(match op {
        BinaryOp::Add => left.wrapping_add(right),
        BinaryOp::Subtract => left.wrapping_sub(right),
        BinaryOp::Multiply => left.wrapping_mul(right),
        BinaryOp::Divide => left.checked_div(right).unwrap_or(0),
        BinaryOp::Remainder => left.checked_rem(right).unwrap_or(0),
        BinaryOp::BitAnd => left & right,
        BinaryOp::BitOr => left | right,
        BinaryOp::BitXor => left ^ right,
        BinaryOp::ShiftLeft => left.checked_shl(u32::try_from(right).ok()?).unwrap_or(0),
        BinaryOp::ShiftRight => left.checked_shr(u32::try_from(right).ok()?).unwrap_or(0),
    }))
}

fn eval_f64(op: BinaryOp, left: f64, right: f64) -> Option<ConstVal> {
    Some(ConstVal::Double(match op {
        BinaryOp::Add => left + right,
        BinaryOp::Subtract => left - right,
        BinaryOp::Multiply => left * right,
        BinaryOp::Divide => left / right,
        BinaryOp::Remainder
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight => return None,
    }))
}

fn compare_order(cc: ConditionCode, ordering: std::cmp::Ordering) -> bool {
    match cc {
        ConditionCode::E => ordering.is_eq(),
        ConditionCode::NE => !ordering.is_eq(),
        ConditionCode::L | ConditionCode::B => ordering.is_lt(),
        ConditionCode::LE | ConditionCode::BE => ordering.is_le(),
        ConditionCode::G | ConditionCode::A => ordering.is_gt(),
        ConditionCode::GE | ConditionCode::AE => ordering.is_ge(),
        ConditionCode::P => false,
    }
}

fn compare_double(cc: ConditionCode, left: f64, right: f64) -> Option<bool> {
    let unordered = left.is_nan() || right.is_nan();
    Some(match cc {
        ConditionCode::E => !unordered && left == right,
        ConditionCode::NE => unordered || left != right,
        ConditionCode::L | ConditionCode::B => !unordered && left < right,
        ConditionCode::LE | ConditionCode::BE => !unordered && left <= right,
        ConditionCode::G | ConditionCode::A => !unordered && left > right,
        ConditionCode::GE | ConditionCode::AE => !unordered && left >= right,
        ConditionCode::P => unordered,
    })
}

fn as_i64(value: ConstVal) -> i64 {
    match value {
        ConstVal::Int(value) => i64::from(value),
        ConstVal::UInt(value) => i64::from(value),
        ConstVal::Long(value) => value,
        ConstVal::ULong(value) => i64::from_ne_bytes(value.to_ne_bytes()),
        ConstVal::Double(value) => value as i64,
    }
}

fn as_u64(value: ConstVal) -> u64 {
    match value {
        ConstVal::Int(value) => value as u64,
        ConstVal::UInt(value) => u64::from(value),
        ConstVal::Long(value) => value as u64,
        ConstVal::ULong(value) => value,
        ConstVal::Double(value) => value as u64,
    }
}

fn as_f64(value: ConstVal) -> f64 {
    match value {
        ConstVal::Int(value) => f64::from(value),
        ConstVal::UInt(value) => f64::from(value),
        ConstVal::Long(value) => value as f64,
        ConstVal::ULong(value) => value as f64,
        ConstVal::Double(value) => value,
    }
}
