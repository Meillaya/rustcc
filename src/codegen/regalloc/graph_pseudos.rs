use crate::codegen::assembly::Operand;
use crate::ir::tacky::{OperandType, TypeEnv};

use super::graph::{InterferenceConfig, InterferenceGraph};
use super::operands::instr_operands;
use super::types::RegisterClass;

pub(super) struct PseudoNodeContext<'a> {
    pub(super) class: RegisterClass,
    pub(super) type_env: &'a TypeEnv,
    pub(super) config: &'a InterferenceConfig,
}

pub(super) fn add_pseudo_nodes(
    graph: &mut InterferenceGraph,
    instructions: &[crate::codegen::assembly::Instr],
    context: &PseudoNodeContext<'_>,
) {
    for instr in instructions {
        for op in instr_operands(instr) {
            let Operand::Pseudo(name) = op else {
                continue;
            };
            if pseudo_is_current_class(&name, context.class, context.type_env)
                && !context.config.static_symbols.contains(&name)
                && !context.config.aliased_pseudos.contains(&name)
            {
                graph.add_node(Operand::Pseudo(name), 0.0);
            }
        }
    }
}

fn pseudo_is_current_class(name: &str, class: RegisterClass, type_env: &TypeEnv) -> bool {
    match (class, type_env.get(name).copied()) {
        (RegisterClass::Gp, Some(OperandType::Double)) => false,
        (RegisterClass::Gp, _) => true,
        (RegisterClass::Xmm, Some(OperandType::Double)) => true,
        (RegisterClass::Xmm, _) => false,
    }
}
