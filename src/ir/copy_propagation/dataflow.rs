use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ir::cfg::{BasicBlock, BlockId, Cfg, NodeId};
use crate::ir::tacky::{Instruction, TypeEnv, Val};

use super::facts::{
    CopyFact, ReachingCopies, ValKey, aggregate_copy_fact, filter_aliased, filter_updated,
    instruction_dst, pointer_base, same_type, update_address_facts, var_is_aliased,
};

type CopyCfg = Cfg<ReachingCopies, Instruction>;
type CopyBlock = BasicBlock<ReachingCopies, Instruction>;

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:121-151.
pub(super) fn find_reaching_copies(
    cfg: Cfg<(), Instruction>,
    type_env: &TypeEnv,
    static_vars: &BTreeSet<String>,
    aliased_vars: &BTreeSet<String>,
) -> CopyCfg {
    let ident = collect_all_copies(&cfg, type_env);
    let mut current_cfg = cfg.initialize_annotation(ident.clone());
    let mut worklist = current_cfg.block_ids().collect::<VecDeque<_>>();

    while let Some(block_id) = worklist.pop_front() {
        let Some(block) = current_cfg.block(block_id).cloned() else {
            continue;
        };
        let old_annotation = block.value.clone();
        let incoming_copies = meet(&ident, &current_cfg, &block);
        let block = transfer(block, incoming_copies, type_env, static_vars, aliased_vars);
        let changed = old_annotation != block.value;
        let succs = block.succs.clone();
        current_cfg.update_basic_block(block);
        if changed {
            enqueue_successors(&mut worklist, succs);
        }
    }
    current_cfg
}

fn enqueue_successors(worklist: &mut VecDeque<BlockId>, succs: Vec<NodeId>) {
    for succ in succs {
        if let NodeId::Block(id) = succ
            && !worklist.contains(&id)
        {
            worklist.push_back(id);
        }
    }
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:101-112.
fn meet(ident: &ReachingCopies, cfg: &CopyCfg, block: &CopyBlock) -> ReachingCopies {
    let mut incoming = ident.clone();
    for pred in &block.preds {
        match pred {
            NodeId::Entry => return ReachingCopies::new(),
            NodeId::Exit => {}
            NodeId::Block(id) => match cfg.get_block_value(*id) {
                Some(value) => incoming = incoming.intersection(value).cloned().collect(),
                None => incoming.clear(),
            },
        }
    }
    incoming
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:114-119.
fn collect_all_copies(cfg: &Cfg<(), Instruction>, type_env: &TypeEnv) -> ReachingCopies {
    let mut address_of = BTreeMap::<String, String>::new();
    let mut copies = ReachingCopies::new();
    for instruction in cfg.cfg_to_instructions() {
        match &instruction {
            Instruction::Copy { src, dst } if same_type(src, dst, type_env) => {
                copies.insert(CopyFact {
                    src: ValKey::from_val(src),
                    dst: ValKey::Var(dst.clone()),
                });
            }
            Instruction::CopyBytes {
                src_pointer,
                dst_pointer,
                ..
            } => {
                if let Some(copy) =
                    aggregate_copy_fact(src_pointer, dst_pointer, &address_of, type_env)
                {
                    copies.insert(copy);
                }
            }
            _ => {}
        }
        update_address_facts(&mut address_of, &instruction);
    }
    copies
}

// Mirrors nqcc2/lib/optimizations/copy_prop.ml:54-99.
fn transfer(
    mut block: CopyBlock,
    initial_reaching_copies: ReachingCopies,
    type_env: &TypeEnv,
    static_vars: &BTreeSet<String>,
    aliased_vars: &BTreeSet<String>,
) -> CopyBlock {
    let mut current_copies = initial_reaching_copies;
    let mut address_of = BTreeMap::<String, String>::new();
    block.instructions = block
        .instructions
        .into_iter()
        .map(|(_, instruction)| {
            let annotation = current_copies.clone();
            current_copies = transfer_instruction(
                &current_copies,
                &instruction,
                TransferCtx {
                    type_env,
                    static_vars,
                    aliased_vars,
                    address_of: &address_of,
                },
            );
            update_address_facts(&mut address_of, &instruction);
            (annotation, instruction)
        })
        .collect();
    block.value = current_copies;
    block
}

struct TransferCtx<'a> {
    type_env: &'a TypeEnv,
    static_vars: &'a BTreeSet<String>,
    aliased_vars: &'a BTreeSet<String>,
    address_of: &'a BTreeMap<String, String>,
}

fn transfer_instruction(
    current_copies: &ReachingCopies,
    instruction: &Instruction,
    ctx: TransferCtx<'_>,
) -> ReachingCopies {
    match instruction {
        Instruction::Copy { src, dst } => transfer_copy(current_copies, src, dst, ctx.type_env),
        Instruction::Call { dst, .. } => {
            transfer_call(current_copies, dst, ctx.static_vars, ctx.aliased_vars)
        }
        Instruction::Store { .. } => {
            filter_aliased(current_copies, ctx.static_vars, ctx.aliased_vars)
        }
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            ..
        } => transfer_copy_bytes(current_copies, src_pointer, dst_pointer, ctx),
        _ => match instruction_dst(instruction) {
            Some(dst) => filter_updated(current_copies, &ValKey::Var(dst)),
            None => current_copies.clone(),
        },
    }
}

fn transfer_copy(
    current_copies: &ReachingCopies,
    src: &Val,
    dst: &str,
    type_env: &TypeEnv,
) -> ReachingCopies {
    let reverse = CopyFact {
        src: ValKey::Var(dst.to_string()),
        dst: ValKey::from_val(src),
    };
    if current_copies.contains(&reverse) {
        current_copies.clone()
    } else if same_type(src, dst, type_env) {
        let mut copies = filter_updated(current_copies, &ValKey::Var(dst.to_string()));
        copies.insert(CopyFact {
            src: ValKey::from_val(src),
            dst: ValKey::Var(dst.to_string()),
        });
        copies
    } else {
        filter_updated(current_copies, &ValKey::Var(dst.to_string()))
    }
}

fn transfer_call(
    current_copies: &ReachingCopies,
    dst: &Option<String>,
    static_vars: &BTreeSet<String>,
    aliased_vars: &BTreeSet<String>,
) -> ReachingCopies {
    let copies = match dst {
        Some(dst) => filter_updated(current_copies, &ValKey::Var(dst.clone())),
        None => current_copies.clone(),
    };
    copies
        .into_iter()
        .filter(|copy| {
            !var_is_aliased(&copy.src, static_vars, aliased_vars)
                && !var_is_aliased(&copy.dst, static_vars, aliased_vars)
        })
        .collect()
}

fn transfer_copy_bytes(
    current_copies: &ReachingCopies,
    src_pointer: &Val,
    dst_pointer: &Val,
    ctx: TransferCtx<'_>,
) -> ReachingCopies {
    let mut copies = match pointer_base(dst_pointer, ctx.address_of) {
        Some(dst) => filter_updated(current_copies, &ValKey::Var(dst.to_string())),
        None => filter_aliased(current_copies, ctx.static_vars, ctx.aliased_vars),
    };
    if let Some(copy) = aggregate_copy_fact(src_pointer, dst_pointer, ctx.address_of, ctx.type_env)
    {
        copies = filter_updated(&copies, &copy.dst);
        copies.insert(copy);
    }
    copies
}
