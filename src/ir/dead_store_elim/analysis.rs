use std::collections::BTreeSet;

use crate::ir::tacky::{Instruction, TackyFunction};

// Mirrors nqcc2/lib/optimizations/address_taken.ml:3-9.
pub(super) fn analyze_address_taken(instructions: &[Instruction]) -> BTreeSet<String> {
    instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::GetAddress { src, .. } => Some(src.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn function_static_storage_vars(
    function: &TackyFunction,
    emitted_static_vars: &BTreeSet<String>,
) -> BTreeSet<String> {
    let local_prefix = format!("{}.", function.name);
    let mut static_vars = emitted_static_vars.clone();
    for name in function.type_env.keys() {
        if is_static_storage_name(name, &local_prefix) {
            static_vars.insert(name.clone());
        }
    }
    static_vars
}

fn is_static_storage_name(name: &str, local_prefix: &str) -> bool {
    !name.starts_with(local_prefix)
        && !name.starts_with("tmp.")
        && !name.starts_with("const.")
        && !name.starts_with("string.")
}
