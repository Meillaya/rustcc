// Mirrors nqcc2/lib/backend/replace_pseudos.ml.
//
// The OCaml `replace_pseudos` pass walks the assembly and rewrites every
// `Pseudo(name)` operand into either a real register (one of the
// callee-saved set chosen by the allocator) or a `Stack(offset)` slot
// relative to `%rbp`. The pass depends on the per-function frame layouts
// produced by `codegen`. The real implementation lands in wave 21
// (chapter 20).
// ch.1 has no pseudoregisters; real replace_pseudos lands in W21 (ch.20).

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;
use crate::codegen::frame::Frame;

/// Replace every `Pseudo` operand with either a physical register or a
/// frame-relative stack slot.
///
/// Chapter 1 input never carries `Pseudo` operands (the only operand
/// shape produced by the chapter-1 codegen is `Operand::Imm` and
/// `Operand::Reg`). Returning the input unchanged is therefore correct
/// for chapter 1 and remains correct until wave 21 introduces temporaries.
pub fn replace_pseudos(asm: AsmProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    Ok(asm)
}
