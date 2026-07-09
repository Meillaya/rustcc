// Mirrors nqcc2/lib/backend/instruction_fixup.ml.
//
// The OCaml `instruction_fixup` pass walks the instruction list for
// each function and rewrites illegal x86-64 forms into explicit
// move + op pairs.  Chapter 9's real fixup covers:
//
//   * Mem-to-mem splits: `movl src, dst` where both operands are
//     memory operands, and `cmpl`/`BinaryOp` with two memory
//     operands.  Route the source through a scratch register
//     (`%r10`) so the emitted instruction is assembler-valid.
//   * Memory-destination `idiv`: `idivl` cannot address memory
//     directly, so route the source through a scratch register.
//   * Memory-destination `cmp`: same restriction as idiv.
//
// Stack-frame allocation lives in `replace_pseudos` (which runs
// before this pass in our pipeline) — the `AllocateStack` it emits
// lives at the start of the function and forms the chapter-9
// prologue.  The emitter is responsible for the
// `pushq %rbp; movq %rsp, %rbp` pair that the `AllocateStack` is
// appended after, and for the canonical `movq %rbp, %rsp; popq
// %rbp; ret` epilogue on each `Ret` instruction.

use anyhow::Result;

use crate::codegen::assembly::{AsmProgram, Instr, TopLevel};

mod split;

use split::split_mem_to_mem;

/// Walk a function's instruction list and split illegal forms.  The
/// split rules are local — they don't introduce new control flow —
/// so a single forward pass is sufficient.
fn fixup_instructions(instructions: Vec<Instr>) -> Vec<Instr> {
    let mut out: Vec<Instr> = Vec::with_capacity(instructions.len());
    for instr in instructions {
        out.extend(split_mem_to_mem(instr));
    }
    out
}

/// Apply chapter-9 fixups to one top-level function.  Returns the
/// rewritten function with mem-to-mem splits applied.
fn fixup_function(func: TopLevel) -> TopLevel {
    let TopLevel::Fn {
        name,
        global,
        instructions,
        type_env,
    } = func
    else {
        return func;
    };
    let fixed = fixup_instructions(instructions);
    TopLevel::Fn {
        name,
        global,
        instructions: fixed,
        type_env,
    }
}

/// Rewrite assembly instructions into forms that are easier to emit
/// and to allocate registers for.
///
/// Chapter 9: real fixup.  For each function in the program,
///   1. Walk the instruction list and split every illegal mem-to-mem
///      form (`movl`, `binaryOp`, `cmpl`, `idivl`) into a
///      `movl mem, %r10; op` pair.
pub fn fixup(asm: AsmProgram, _frames: &[crate::codegen::frame::Frame]) -> Result<AsmProgram> {
    let top_level = asm.top_level.into_iter().map(fixup_function).collect();
    Ok(AsmProgram { top_level })
}
