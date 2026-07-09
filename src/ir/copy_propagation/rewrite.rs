use std::collections::BTreeMap;

use crate::ir::cfg::{BasicBlock, Cfg};
use crate::ir::tacky::{Instruction, TypeEnv, Val};

use super::dataflow::find_reaching_copies;
use super::facts::{CopyFact, ReachingCopies, ValKey, aggregate_copy_fact, update_address_facts};

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:209-227.
pub(super) fn optimize(
    cfg: Cfg<(), Instruction>,
    type_env: &TypeEnv,
    static_vars: &std::collections::BTreeSet<String>,
    aliased_vars: &std::collections::BTreeSet<String>,
) -> Cfg<(), Instruction> {
    let annotated_cfg = find_reaching_copies(cfg, type_env, static_vars, aliased_vars);
    let basic_blocks = annotated_cfg
        .basic_blocks
        .iter()
        .map(|block| rewrite_block(block, type_env))
        .collect();
    Cfg {
        basic_blocks,
        entry: annotated_cfg.entry,
        exit: annotated_cfg.exit,
        entry_succs: annotated_cfg.entry_succs,
        exit_preds: annotated_cfg.exit_preds,
        debug_label: annotated_cfg.debug_label,
    }
}

fn rewrite_block(
    block: &BasicBlock<ReachingCopies, Instruction>,
    type_env: &TypeEnv,
) -> BasicBlock<(), Instruction> {
    let mut address_of = BTreeMap::<String, String>::new();
    let instructions = block
        .instructions
        .iter()
        .filter_map(|(copies, instruction)| {
            let rewritten = rewrite_instruction(copies, instruction, &address_of, type_env);
            update_address_facts(&mut address_of, instruction);
            rewritten
        })
        .collect();
    BasicBlock {
        id: block.id,
        instructions,
        preds: block.preds.clone(),
        succs: block.succs.clone(),
        value: (),
    }
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:153-207.
fn rewrite_instruction(
    copies: &ReachingCopies,
    instruction: &Instruction,
    address_of: &BTreeMap<String, String>,
    type_env: &TypeEnv,
) -> Option<((), Instruction)> {
    if redundant_aggregate_copy(copies, instruction, address_of, type_env) {
        return None;
    }
    if redundant_scalar_copy(copies, instruction) {
        return None;
    }
    Some(((), replace_instruction_sources(instruction, copies)))
}

fn redundant_aggregate_copy(
    copies: &ReachingCopies,
    instruction: &Instruction,
    address_of: &BTreeMap<String, String>,
    type_env: &TypeEnv,
) -> bool {
    let Instruction::CopyBytes {
        src_pointer,
        dst_pointer,
        ..
    } = instruction
    else {
        return false;
    };
    let Some(copy) = aggregate_copy_fact(src_pointer, dst_pointer, address_of, type_env) else {
        return false;
    };
    let reverse = CopyFact {
        src: copy.dst.clone(),
        dst: copy.src.clone(),
    };
    copies.contains(&copy) || copies.contains(&reverse)
}

fn redundant_scalar_copy(copies: &ReachingCopies, instruction: &Instruction) -> bool {
    let Instruction::Copy { src, dst } = instruction else {
        return false;
    };
    let copy = CopyFact {
        src: ValKey::from_val(src),
        dst: ValKey::Var(dst.clone()),
    };
    let reverse = CopyFact {
        src: ValKey::Var(dst.clone()),
        dst: ValKey::from_val(src),
    };
    copies.contains(&copy) || copies.contains(&reverse)
}

fn replace_instruction_sources(instruction: &Instruction, copies: &ReachingCopies) -> Instruction {
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
        Instruction::Negate { .. }
        | Instruction::Complement { .. }
        | Instruction::Not { .. }
        | Instruction::Jump { .. }
        | Instruction::Label(_)
        | Instruction::GetAddress { .. } => instruction.clone(),
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
