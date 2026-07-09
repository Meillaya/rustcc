use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::ir::tacky::{Instruction, OperandType, TypeEnv, Val};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CopyFact {
    pub(super) src: ValKey,
    pub(super) dst: ValKey,
}

impl Ord for CopyFact {
    fn cmp(&self, other: &Self) -> Ordering {
        self.src
            .cmp(&other.src)
            .then_with(|| self.dst.cmp(&other.dst))
    }
}

impl PartialOrd for CopyFact {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ValKey {
    Constant(i64),
    Var(String),
    ConstantDouble(u64),
}

impl ValKey {
    pub(super) fn from_val(val: &Val) -> Self {
        match val {
            Val::Constant(value) => Self::Constant(*value),
            Val::Var(name) => Self::Var(name.clone()),
            Val::ConstantDouble(value) => Self::ConstantDouble(value.to_bits()),
        }
    }

    pub(super) fn to_val(&self) -> Val {
        match self {
            Self::Constant(value) => Val::Constant(*value),
            Self::Var(name) => Val::Var(name.clone()),
            Self::ConstantDouble(bits) => Val::ConstantDouble(f64::from_bits(*bits)),
        }
    }
}

pub(super) type ReachingCopies = BTreeSet<CopyFact>;

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:49-52.
pub(super) fn filter_updated(copies: &ReachingCopies, updated: &ValKey) -> ReachingCopies {
    copies
        .iter()
        .filter(|copy| copy.src != *updated && copy.dst != *updated)
        .cloned()
        .collect()
}

pub(super) fn filter_aliased(
    copies: &ReachingCopies,
    static_vars: &BTreeSet<String>,
    aliased_vars: &BTreeSet<String>,
) -> ReachingCopies {
    copies
        .iter()
        .filter(|copy| {
            !var_is_aliased(&copy.src, static_vars, aliased_vars)
                && !var_is_aliased(&copy.dst, static_vars, aliased_vars)
        })
        .cloned()
        .collect()
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:37-40.
pub(super) fn same_type(src: &Val, dst: &str, type_env: &TypeEnv) -> bool {
    let dst_ty = var_type(dst, type_env);
    match src {
        Val::Var(name) => compatible_types(var_type(name, type_env), dst_ty),
        Val::Constant(value) => {
            matches!(dst_ty, OperandType::Int | OperandType::UInt)
                || (*value == 0 && is_integer_scalar(dst_ty))
        }
        Val::ConstantDouble(_) => dst_ty == OperandType::Double,
    }
}

pub(super) fn compatible_types(src_ty: OperandType, dst_ty: OperandType) -> bool {
    src_ty == dst_ty
        || (is_integer_scalar(src_ty)
            && is_integer_scalar(dst_ty)
            && src_ty.is_unsigned() == dst_ty.is_unsigned())
        || matches!(
            (src_ty, dst_ty),
            (OperandType::ByteArray { size: left }, OperandType::ByteArray { size: right }) if left == right
        )
}

fn is_integer_scalar(ty: OperandType) -> bool {
    matches!(
        ty,
        OperandType::Int
            | OperandType::UInt
            | OperandType::Byte
            | OperandType::UByte
            | OperandType::Long
            | OperandType::ULong
    )
}

pub(super) fn var_type(name: &str, type_env: &TypeEnv) -> OperandType {
    type_env.get(name).copied().unwrap_or(OperandType::Int)
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:42-47.
pub(super) fn var_is_aliased(
    val: &ValKey,
    static_vars: &BTreeSet<String>,
    aliased_vars: &BTreeSet<String>,
) -> bool {
    match val {
        ValKey::Var(name) => static_vars.contains(name) || aliased_vars.contains(name),
        ValKey::Constant(_) | ValKey::ConstantDouble(_) => false,
    }
}

pub(super) fn aggregate_copy_fact(
    src_pointer: &Val,
    dst_pointer: &Val,
    address_of: &BTreeMap<String, String>,
    type_env: &TypeEnv,
) -> Option<CopyFact> {
    let src = pointer_base(src_pointer, address_of)?;
    let dst = pointer_base(dst_pointer, address_of)?;
    compatible_types(var_type(src, type_env), var_type(dst, type_env)).then(|| CopyFact {
        src: ValKey::Var(src.to_string()),
        dst: ValKey::Var(dst.to_string()),
    })
}

pub(super) fn pointer_base<'a>(
    val: &'a Val,
    address_of: &'a BTreeMap<String, String>,
) -> Option<&'a str> {
    match val {
        Val::Var(name) => address_of.get(name).map(String::as_str),
        Val::Constant(_) | Val::ConstantDouble(_) => None,
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

pub(super) fn update_address_facts(
    address_of: &mut BTreeMap<String, String>,
    instruction: &Instruction,
) {
    if let Some(dst) = instruction_dst(instruction) {
        address_of.remove(&dst);
    }
    if let Instruction::GetAddress { src, dst } = instruction {
        address_of.insert(dst.clone(), src.clone());
    }
}
