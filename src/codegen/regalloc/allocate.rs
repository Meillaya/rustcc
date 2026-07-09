use std::collections::{BTreeSet, HashSet};

use anyhow::Result;

use crate::codegen::assembly::{AsmProgram, Instr, Operand, Reg, TopLevel};
use crate::driver::RegallocOptions;

use super::abi_liveness::conservative_liveness_config;
use super::coalesce::coalesce_once;
use super::rewrite::{cleanup_redundant_moves, replace_colored_pseudos};
use super::scratch::use_reserved_address_scratch;
use super::spill::{SpillState, max_reallocation_passes};
use crate::ir::tacky::TypeEnv;

use super::graph::{InterferenceBuild, InterferenceConfig, InterferenceGraph, build_interference};
use super::simplify::simplify;
use super::types::{LivenessConfig, RegisterClass};
use super::{analyze_function_liveness, select};

struct AllocationInput<'a> {
    fn_name: &'a str,
    instructions: &'a [Instr],
    type_env: &'a TypeEnv,
    globals: &'a HashSet<String>,
    class: RegisterClass,
    options: RegallocOptions,
}

struct FunctionAllocation {
    instructions: Vec<Instr>,
    used_callee_saved: BTreeSet<Reg>,
}

struct SelectPass {
    instructions: Vec<Instr>,
    selected: super::SelectResult,
}

pub fn allocate(
    asm: AsmProgram,
    globals: &HashSet<String>,
    options: RegallocOptions,
) -> Result<AsmProgram> {
    let top_level = asm
        .top_level
        .into_iter()
        .map(|item| allocate_top_level(item, globals, options))
        .collect::<Result<Vec<_>>>()?;
    Ok(AsmProgram { top_level })
}

fn allocate_top_level(
    item: TopLevel,
    globals: &HashSet<String>,
    options: RegallocOptions,
) -> Result<TopLevel> {
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
        options,
    })?;
    let xmm = allocate_class(AllocationInput {
        fn_name: &name,
        instructions: &gp.instructions,
        type_env: &type_env,
        globals,
        class: RegisterClass::Xmm,
        options,
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
    let mut spill_state = SpillState::from_stack_only(input.instructions);
    let max_passes = max_reallocation_passes(input.instructions);

    // Mirrors nqcc2/lib/backend/regalloc.ml:595-620: build the graph, color it,
    // leave spilled pseudos for stack replacement, and retry with those pseudos
    // forced out of the graph so allocation reaches a spill-free fixed point.
    for _ in 1..=max_passes {
        let pass = select_class_pass(&input, &spill_state)?;
        let new_spills = spill_state.add_coloring_spills(&pass.selected.assignments);
        if new_spills == 0 {
            return Ok(FunctionAllocation {
                instructions: replace_colored_pseudos(
                    &pass.instructions,
                    &pass.selected.assignments,
                    input.class,
                ),
                used_callee_saved: pass.selected.used_callee_saved_regs,
            });
        }
    }

    anyhow::bail!(
        "register allocation for {} exceeded {max_passes} spill passes",
        input.class.name()
    )
}

fn select_class_pass(input: &AllocationInput<'_>, spill_state: &SpillState) -> Result<SelectPass> {
    let interference = InterferenceConfig {
        aliased_pseudos: spill_state.pseudos().clone(),
        static_symbols: input.globals.iter().cloned().collect(),
    };
    let liveness_config = conservative_liveness_config(input.instructions);
    let (graph, instructions) = if input.options.coalescing_enabled {
        let mut instructions = input.instructions.to_vec();
        loop {
            let graph = build_class_graph(input, &instructions, &interference, &liveness_config)?;
            let (coalesced_graph, rewritten, changed) =
                coalesce_once(graph, &instructions, input.class);
            if !changed {
                break (coalesced_graph, rewritten);
            }
            instructions = rewritten;
        }
    } else {
        (
            build_class_graph(input, input.instructions, &interference, &liveness_config)?,
            input.instructions.to_vec(),
        )
    };
    let selected = select(&graph, &simplify(&graph));
    Ok(SelectPass {
        instructions,
        selected,
    })
}

fn build_class_graph(
    input: &AllocationInput<'_>,
    instructions: &[Instr],
    interference: &InterferenceConfig,
    liveness_config: &LivenessConfig,
) -> Result<InterferenceGraph> {
    let liveness =
        analyze_function_liveness(input.fn_name, instructions, input.class, liveness_config)?;
    build_interference(InterferenceBuild {
        instructions,
        liveness_cfg: &liveness,
        class: input.class,
        type_env: input.type_env,
        interference,
        liveness: liveness_config,
    })
    .map_err(Into::into)
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
