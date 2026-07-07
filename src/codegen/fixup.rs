// Mirrors nqcc2/lib/backend/instruction_fixup.ml.
//
// The OCaml `instruction_fixup` pass rewrites two-operand assembly forms
// that are syntactically inconvenient (`mov $5, $4`, `binaryOp op X, X`,
// `div X`, `idiv X`) into explicit move + op pairs and into the `cqo` /
// `cdq` sign-extension setup that x86-64 division needs. The pass also
// tracks which instructions define `%rax` / `%rdx` so the allocator can
// safely coalesce. The real implementation lands in wave 10 (chapter 9).
// ch.1 has no fixups; this is identity. Real fixups land in W10 (ch.9+).

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;
use crate::codegen::frame::Frame;

/// Rewrite assembly instructions into forms that are easier to emit and to
/// allocate registers for. Returns the (possibly modified) program.
///
/// Chapter 1 input contains only `Mov {Imm -> AX}` and `Ret`, neither of
/// which is on the OCaml `instruction_fixup` rewriter list (mov with two
/// immediates, binary op with same src/dst, `idiv`). Returning the input
/// unchanged is therefore correct for chapter 1 and remains correct until
/// wave 10 introduces binary ops and division.
pub fn fixup(asm: AsmProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    Ok(asm)
}
