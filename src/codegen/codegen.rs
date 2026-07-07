// Mirrors nqcc2/lib/backend/codegen.ml.
//
// The OCaml file owns TACKY -> assembly conversion: it walks each TACKY
// function and emits an `AsmProgram`.  Chapter 1 covered the
// `Return(Constant n) -> [Mov, Ret]` arm; chapter 2 widens the surface
// with the pseudoregister-based unary pipeline that matches the OCaml
// `convert_instruction` switch:
//
//   Return v                 -> [ Mov(v, Reg AX); Ret ]
//   Copy { src; dst: v }     -> [ Mov(src, Pseudo(v)) ]
//   Negate { dst: v }        -> [ Unary(Neg, Pseudo(v)) ]
//   Complement { dst: v }    -> [ Unary(Not, Pseudo(v)) ]
//
// The pseudoregisters survive the codegen pass and are resolved into
// `%rbp`-relative `Stack(offset)` operands by `replace_pseudos` once
// the function has walked its frame.  Frames stay empty for chapter 2
// because the only locals we allocate are temporaries; the parameter
// stays so the pipeline signature remains stable across waves.

use anyhow::Result;

use crate::codegen::assembly::{AsmProgram, Instr, Operand, Reg, TopLevel, UnaryOpInstr};
use crate::codegen::frame::Frame;
use crate::ir::tacky::{Instruction, TackyProgram, Val};

fn convert_val(val: &Val) -> Operand {
    match val {
        Val::Constant(n) => Operand::Imm(*n),
        Val::Var(name) => Operand::Pseudo(name.clone()),
    }
}

fn lower_instruction(instr: &Instruction) -> Vec<Instr> {
    match instr {
        Instruction::Return(val) => vec![
            Instr::Mov {
                src: convert_val(val),
                dst: Operand::Reg(Reg::AX),
            },
            Instr::Ret,
        ],
        Instruction::Copy { src, dst } => vec![Instr::Mov {
            src: convert_val(src),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::Negate { dst } => vec![Instr::Unary {
            op: UnaryOpInstr::Neg,
            operand: Operand::Pseudo(dst.clone()),
        }],
        Instruction::Complement { dst } => vec![Instr::Unary {
            op: UnaryOpInstr::Not,
            operand: Operand::Pseudo(dst.clone()),
        }],
        _ => Vec::new(),
    }
}

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
