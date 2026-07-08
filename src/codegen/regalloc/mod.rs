// Mirrors nqcc2/lib/backend/regalloc.ml (651 LOC; uses Briggs/George coalescing).
//
// The OCaml register allocator is the largest file in the backend:
// it builds an interference graph from the assembly, simplifies it via
// the Briggs / George algorithm, then selects registers and emits
// spills for nodes that ran out of colors. Future waves may split this
// module into submodules (`interference.rs`, `color.rs`, `spill.rs`,
// `coalesce.rs`); the current single-file layout keeps the scaffold
// cheap to navigate. The real implementation lands in wave 21
// (chapter 20).
#![allow(dead_code)]

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;

/// Assign a physical register to every `Reg` use in the assembly, spilling
/// when the available callee-saved set is exhausted.
pub fn allocate(_asm: AsmProgram) -> Result<AsmProgram> {
    unimplemented!("ch.20 regalloc wired in wave 21")
}
