use std::collections::{BTreeMap, BTreeSet, HashSet};

use anyhow::Result;

use crate::codegen::assembly::{AsmProgram, Instr, Operand, Reg, TopLevel};

use super::rewrite::{cleanup_redundant_moves, replace_colored_pseudos};
use super::scratch::use_reserved_address_scratch;
use crate::ir::tacky::TypeEnv;

use super::graph::{InterferenceBuild, InterferenceConfig, build_interference};
use super::simplify::simplify;
use super::types::{LivenessConfig, RegisterClass};
use super::{analyze_function_liveness, select};

struct AllocationInput<'a> {
    fn_name: &'a str,
    instructions: &'a [Instr],
    type_env: &'a TypeEnv,
    globals: &'a HashSet<String>,
    class: RegisterClass,
}

struct FunctionAllocation {
    instructions: Vec<Instr>,
    used_callee_saved: BTreeSet<Reg>,
}

pub fn allocate(asm: AsmProgram, globals: &HashSet<String>) -> Result<AsmProgram> {
    let top_level = asm
        .top_level
        .into_iter()
        .map(|item| allocate_top_level(item, globals))
        .collect::<Result<Vec<_>>>()?;
    Ok(AsmProgram { top_level })
}

fn allocate_top_level(item: TopLevel, globals: &HashSet<String>) -> Result<TopLevel> {
    let TopLevel::Fn {
        name,
        global,
        instructions,
        type_env,
    } = item
    else {
        return Ok(item);
    };
    let instructions = use_reserved_address_scratch(&instructions);
    let gp = allocate_class(AllocationInput {
        fn_name: &name,
        instructions: &instructions,
        type_env: &type_env,
        globals,
        class: RegisterClass::Gp,
    })?;
    let xmm = allocate_class(AllocationInput {
        fn_name: &name,
        instructions: &gp.instructions,
        type_env: &type_env,
        globals,
        class: RegisterClass::Xmm,
    })?;
    let instructions = preserve_callee_saved(xmm.instructions, &gp.used_callee_saved);
    let instructions = cleanup_redundant_moves(instructions);
    Ok(TopLevel::Fn {
        name,
        global,
        instructions,
        type_env,
    })
}

fn allocate_class(input: AllocationInput<'_>) -> Result<FunctionAllocation> {
    let liveness_config = conservative_liveness_config(input.instructions);
    let liveness = analyze_function_liveness(
        input.fn_name,
        input.instructions,
        input.class,
        &liveness_config,
    )?;
    let interference = InterferenceConfig {
        aliased_pseudos: stack_only_pseudos(input.instructions),
        static_symbols: input.globals.iter().cloned().collect(),
    };
    let graph = build_interference(InterferenceBuild {
        instructions: input.instructions,
        liveness_cfg: &liveness,
        class: input.class,
        type_env: input.type_env,
        interference: &interference,
        liveness: &liveness_config,
    })?;
    let selected = select(&graph, &simplify(&graph));
    Ok(FunctionAllocation {
        instructions: replace_colored_pseudos(
            input.instructions,
            &selected.assignments,
            input.class,
        ),
        used_callee_saved: selected.used_callee_saved_regs,
    })
}

fn stack_only_pseudos(instructions: &[Instr]) -> BTreeSet<String> {
    let mut pseudos = BTreeSet::new();
    for instr in instructions {
        match instr {
            Instr::Lea {
                src: Operand::Pseudo(name),
                ..
            }
            | Instr::Lea {
                src: Operand::PseudoMem(name, _),
                ..
            } => {
                pseudos.insert(name.clone());
            }
            _ => collect_pseudomem(instr, &mut pseudos),
        }
    }
    pseudos
}

fn collect_pseudomem(instr: &Instr, pseudos: &mut BTreeSet<String>) {
    for operand in super::instr_operands(instr) {
        if let Operand::PseudoMem(name, _) = operand {
            pseudos.insert(name);
        }
    }
}

fn conservative_liveness_config(instructions: &[Instr]) -> LivenessConfig {
    let mut call_param_regs = BTreeMap::<String, BTreeSet<Reg>>::new();
    for (index, instr) in instructions.iter().enumerate() {
        if let Instr::Call(name) = instr {
            call_param_regs
                .entry(name.clone())
                .or_default()
                .extend(call_regs_before(instructions, index));
        }
    }
    LivenessConfig {
        return_regs: return_regs_before_rets(instructions).into_iter().collect(),
        call_param_regs: call_param_regs
            .into_iter()
            .map(|(name, regs)| (name, regs.into_iter().collect()))
            .collect(),
    }
}

fn call_regs_before(instructions: &[Instr], call_index: usize) -> BTreeSet<Reg> {
    let mut regs = BTreeSet::new();
    for instr in instructions[..call_index].iter().rev() {
        match instr {
            Instr::Push(_) | Instr::AllocateStack(_) => {}
            _ => {
                let Some(reg) = written_reg(instr) else {
                    break;
                };
                if !param_regs().contains(&reg) {
                    break;
                }
                regs.insert(reg);
            }
        }
    }
    regs
}

fn return_regs_before_rets(instructions: &[Instr]) -> BTreeSet<Reg> {
    let mut regs = BTreeSet::new();
    for (index, instr) in instructions.iter().enumerate() {
        if !matches!(instr, Instr::Ret) {
            continue;
        }
        for prior in instructions[..index].iter().rev() {
            let Some(reg) = written_reg(prior) else {
                break;
            };
            if !return_regs().contains(&reg) {
                break;
            }
            regs.insert(reg);
        }
    }
    regs
}

fn written_reg(instr: &Instr) -> Option<Reg> {
    match instr {
        Instr::Mov { dst, .. }
        | Instr::Movq { dst, .. }
        | Instr::MovByte { dst, .. }
        | Instr::Movsx { dst, .. }
        | Instr::MovZeroExtend { dst, .. }
        | Instr::MovSignExtendByte { dst, .. }
        | Instr::Movsd { dst, .. }
        | Instr::MovsdLoad { dst, .. }
        | Instr::Lea { dst, .. }
        | Instr::Cvtsi2sd { dst, .. }
        | Instr::Cvttsd2si { dst, .. }
        | Instr::SetCC { dst, .. } => match dst {
            Operand::Reg(reg) => Some(reg.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn param_regs() -> BTreeSet<Reg> {
    [
        Reg::DI,
        Reg::SI,
        Reg::DX,
        Reg::CX,
        Reg::R8,
        Reg::R9,
        Reg::XMM(0),
        Reg::XMM(1),
        Reg::XMM(2),
        Reg::XMM(3),
        Reg::XMM(4),
        Reg::XMM(5),
        Reg::XMM(6),
        Reg::XMM(7),
    ]
    .into_iter()
    .collect()
}

fn return_regs() -> BTreeSet<Reg> {
    [Reg::AX, Reg::DX, Reg::XMM(0), Reg::XMM(1)]
        .into_iter()
        .collect()
}

fn preserve_callee_saved(instructions: Vec<Instr>, used: &BTreeSet<Reg>) -> Vec<Instr> {
    if used.is_empty() {
        return instructions;
    }
    let needs_padding = used.len() % 2 == 1;
    let mut output =
        Vec::with_capacity(instructions.len() + used.len() * 2 + usize::from(needs_padding) * 2);
    if needs_padding {
        output.push(Instr::AllocateStack(8));
    }
    output.extend(
        used.iter()
            .cloned()
            .map(|reg| Instr::Push(Operand::Reg(reg))),
    );
    for instr in instructions {
        match instr {
            Instr::Ret => {
                output.extend(used.iter().rev().cloned().map(Instr::Pop));
                if needs_padding {
                    output.push(Instr::DeallocateStack(8));
                }
                output.push(Instr::Ret);
            }
            other => output.push(other),
        }
    }
    output
}
