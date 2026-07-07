// Mirrors nqcc2/lib/backend/replace_pseudos.ml.
//
// The OCaml `replace_pseudos` pass walks the assembly and rewrites every
// `Pseudo(name)` operand into either a real register (one of the
// callee-saved set chosen by the allocator) or a `Stack(offset)` slot
// relative to `%rbp`. The pass depends on the per-function frame layouts
// produced by `codegen`. The real implementation lands in wave 21
// (chapter 20).
#![allow(dead_code)]

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;
use crate::codegen::frame::Frame;

/// Replace every `Pseudo` operand with either a physical register or a
/// frame-relative stack slot.
pub fn replace_pseudos(_asm: AsmProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    unimplemented!("ch.20 replace wired in wave 21")
}