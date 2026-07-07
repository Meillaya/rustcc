// Mirrors nqcc2/lib/backend/instruction_fixup.ml.
//
// The OCaml `instruction_fixup` pass rewrites two-operand assembly forms
// that are syntactically inconvenient (`mov $5, $4`, `binaryOp op X, X`,
// `div X`, `idiv X`) into explicit move + op pairs and into the `cqo` /
// `cdq` sign-extension setup that x86-64 division needs. The pass also
// tracks which instructions define `%rax` / `%rdx` so the allocator can
// safely coalesce. The real implementation lands in wave 10 (chapter 9).
#![allow(dead_code)]

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;
use crate::codegen::frame::Frame;

/// Rewrite assembly instructions into forms that are easier to emit and to
/// allocate registers for. Returns the (possibly modified) program.
pub fn fixup(_asm: AsmProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    unimplemented!("ch.9+ fixup wired in wave 10")
}