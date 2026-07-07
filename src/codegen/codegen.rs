// Mirrors nqcc2/lib/backend/codegen.ml.
//
// The OCaml file owns TACKY -> assembly conversion: it walks each TACKY
// function, computes a stack frame, and emits an `AsmProgram`. The wave-2
// landing delivers the chapter-1 subset: the OCaml
// `convert_top_level :: Tacky.Function -> Assembly.Function` arm for a
// function whose body is exactly `Return(Constant n)` is mirrored here.
//
// Chapter-1 algorithm (per function):
//   1. Each `Instruction::Return(Val::Constant n)` lowers to two assembly
//      instructions: `movl $n, %eax` followed by `ret`. The OCaml
//      `convert_return_instruction`/`Assembly.Ret` pair produces the same
//      effect (line 480 of `codegen.ml` returns `int_retvals @ [ Ret ]`).
//   2. Non-`Return` instructions lower to an empty slice. The book grows
//      this in chapter 2+ via the `convert_instruction` switch.
//   3. The resulting instructions, plus the `global = (name == "main")`
//      flag from the OCaml `convert_top_level` arm, are wrapped in one
//      `TopLevel::Fn` entry per TACKY function.
//
// Later waves (3+) add prologue/epilogue emission, callee-setup, frame
// layout, and the full `convert_instruction` switch; the interface stays
// stable because `frames: &[Frame]` is already reserved for chapter-7+ use.

use anyhow::Result;

use crate::codegen::assembly::{AsmProgram, Instr, Operand, Reg, TopLevel};
use crate::codegen::frame::Frame;
use crate::ir::tacky::{Instruction, TackyProgram, Val};

/// Lower a single TACKY instruction into a flat assembly instruction slice.
///
/// Chapter-1 only handles `Return(Constant n)` -> `[Mov, Ret]`. Every other
/// instruction lowers to an empty slice; the lowering grows incrementally
/// per the book's chapter progression (return -> unary -> binary -> control
/// flow -> locals -> structs ...).
fn lower_instruction(instr: &Instruction) -> Vec<Instr> {
    match instr {
        Instruction::Return(Val::Constant(n)) => vec![
            Instr::Mov {
                src: Operand::Imm(*n),
                dst: Operand::Reg(Reg::AX),
            },
            Instr::Ret,
        ],
        // ch.1 has no other TACKY forms; the OCaml `convert_instruction`
        // switch adds them chapter-by-chapter from W2-T3 onwards.
        Instruction::Return(Val::Var(_)) => Vec::new(),
        _ => Vec::new(),
    }
}

/// Generate an assembly program from a TACKY input.
///
/// Walks each `TackyFunction` and produces one `TopLevel::Fn { global,
/// instructions }` entry. The OCaml side names the matching entry point
/// `gen :: Tacky.Program -> Assembly.Program`; the Rust port stages the
/// constant lowering through `lower_instruction` for the chapter-1 subset.
/// `frames` is unused for chapter 1 (the stack frame is empty until chapter
/// 7+ introduces locals and callee arguments) but the parameter stays so
/// the pipeline signature remains stable across waves.
pub fn generate(tacky: &TackyProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    let top_level = tacky
        .functions
        .iter()
        .map(|func| {
            let instructions = func
                .body
                .iter()
                .flat_map(lower_instruction)
                .collect::<Vec<_>>();
            TopLevel::Fn {
                name: func.name.clone(),
                global: func.name == "main",
                instructions,
            }
        })
        .collect();

    Ok(AsmProgram { top_level })
}
