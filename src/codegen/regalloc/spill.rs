// Mirrors nqcc2/lib/backend/regalloc.ml:563-620 and replace_pseudos.ml:1-137.
//
// Coloring leaves uncolored pseudos as spill decisions. This module records
// those decisions, keeps spilled pseudos out of later interference graphs, and
// lets the existing replace-pseudos pass assign the concrete frame slots.

use std::collections::{BTreeMap, BTreeSet};

use crate::codegen::assembly::{Instr, Operand, Reg};

use super::color::ColorMap;
use super::operands::instr_operands;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpillState {
    pseudos: BTreeSet<String>,
}

impl SpillState {
    pub(crate) fn from_stack_only(instructions: &[Instr]) -> Self {
        Self {
            pseudos: stack_only_pseudos(instructions),
        }
    }

    pub(crate) fn pseudos(&self) -> &BTreeSet<String> {
        &self.pseudos
    }

    pub(crate) fn add_coloring_spills(&mut self, assignments: &ColorMap) -> usize {
        coloring_spills(assignments)
            .into_iter()
            .filter(|pseudo| self.pseudos.insert(pseudo.clone()))
            .count()
    }
}

pub(crate) fn max_reallocation_passes(instructions: &[Instr]) -> usize {
    all_pseudos(instructions).len().saturating_add(1).max(1)
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
    for operand in instr_operands(instr) {
        if let Operand::PseudoMem(name, _) = operand {
            pseudos.insert(name);
        }
    }
}

fn coloring_spills(assignments: &BTreeMap<Operand, Option<Reg>>) -> BTreeSet<String> {
    assignments
        .iter()
        .filter_map(|(operand, reg)| match (operand, reg) {
            (Operand::Pseudo(name), None) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn all_pseudos(instructions: &[Instr]) -> BTreeSet<String> {
    instructions
        .iter()
        .flat_map(instr_operands)
        .filter_map(|operand| match operand {
            Operand::Pseudo(name) | Operand::PseudoMem(name, _) => Some(name),
            _ => None,
        })
        .collect()
}
