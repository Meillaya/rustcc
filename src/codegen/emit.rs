// Mirrors nqcc2/lib/emit.ml.
//
// The OCaml emitter walks the `AsmProgram` and produces x86-64 AT&T
// assembly text. This module delivers the chapter-1 subset:
//
//   .globl main
//   main:
//       movl $2, %eax
//       ret
//
// Indentation is four spaces per the OCaml `emit_instruction` preamble
// (`\tmov%s %s, %s\n`). Each line joins with `\n` and the program ends
// with a trailing `\n` so the file is line-terminated like every other
// hand-written or book-emitted `.s` source.
//
// Later waves (3+) extend `format_instruction` for unary/binary ops, the
// prologue/epilogue (`pushq %rbp` / `movq %rsp, %rbp`), `Ret` with the
// stack-restore trio, and `Pseudo`/`Stack` operand rendering. The
// OCaml-driven layout (one top-level per block, function prologue
// emitted by `emit_tl Function`) lands in W3-T1 once chapters 2+ codegen
// starts producing real frames.

use anyhow::{Result, anyhow};

use crate::codegen::assembly::{AsmProgram, Instr, Operand, Reg, TopLevel};

const INDENT: &str = "    ";

/// Format a single assembly instruction as a single line (no trailing newline).
///
/// The chapter-1 surface only handles `Mov { Imm -> AX }` and `Ret`. Both
/// mirror their OCaml counterparts in `emit_instruction` (lines 177-179
/// for `Mov`, 237-242 for `Ret`). Returning `Err` for unhandled variants
/// surfaces regression: a later wave that adds a TACKY instruction without
/// extending this function will trip here during testing instead of
/// silently producing malformed assembly.
fn format_instruction(instr: &Instr) -> Result<String> {
    match instr {
        Instr::Mov {
            src: Operand::Imm(n),
            dst: Operand::Reg(Reg::AX),
        } => Ok(format!("movl ${n}, %eax")),
        Instr::Mov { .. } => Err(anyhow!(
            "ch.1 only emits Mov {{Imm -> AX}}; replace_pseudos lands in W21"
        )),
        Instr::Ret => Ok("ret".to_string()),
        other => Err(anyhow!(
            "ch.1 emit does not support instruction variant; land it in a later wave: {other:?}"
        )),
    }
}

/// Format one top-level `Fn` block as a multi-line text.
///
/// Mirrors the OCaml `emit_tl Function` arm (line 302 of `emit.ml`). The
/// OCaml version inserts `.text`, `pushq %rbp`, and `movq %rsp, %rbp`
/// before the instructions; for chapter 1 only the global directive, the
/// label line, and the (indented) instruction lines are emitted. The
/// prologue is added when chapter 3+ codegen ships real frames.
fn format_function(name: &str, global: bool, instructions: &[Instr]) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();
    if global {
        lines.push(format!(".globl {name}"));
    }
    lines.push(format!("{name}:"));
    for instr in instructions {
        lines.push(format!("{INDENT}{}", format_instruction(instr)?));
    }
    Ok(lines.join("\n"))
}

/// Pretty-print an `AsmProgram` to x86-64 AT&T assembly text.
///
/// Top-level items are concatenated in source order, separated by a single
/// blank line between functions (matches `nqcc2/lib/emit.ml` line 312 +
/// the GNU `as` convention of separating procedures with blank lines).
/// The final newline keeps the file POSIX-terminated so `gcc` does not
/// emit a "trailing newline missing" diagnostic.
pub fn emit(program: &AsmProgram) -> Result<String> {
    let mut blocks: Vec<String> = Vec::new();
    for (idx, item) in program.top_level.iter().enumerate() {
        let block = match item {
            TopLevel::Fn {
                name,
                global,
                instructions,
            } => format_function(name, *global, instructions)?,
            TopLevel::StaticVariable { .. } | TopLevel::Constant { .. } => {
                return Err(anyhow!(
                    "ch.1 only emits Fn top-levels; data sections land in W12+"
                ));
            }
        };
        // Skip the separator before the very first block.
        if idx == 0 {
            blocks.push(block);
        } else {
            blocks.push(format!("\n{block}"));
        }
    }
    blocks.push(String::new()); // trailing newline
    Ok(blocks.join("\n"))
}
