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
// Chapter 3 adds the binary arithmetic / bitwise / shift pipeline.  The
// lowerer emits the two-address pattern `Copy left, tmp;
// BinaryOp { src: right, tmp }` so the destination already holds the
// left operand by the time `BinaryOp` is converted.  The standard
// arithmetic / bitwise binops therefore collapse to a single
// `<op> src, dst` instruction (the canonical x86 two-address shape);
// division / remainder use the %eax:%edx pair that `idivl` requires;
// shifts use %cl as the count register per the book's chapter 3.
//
//   Add/Sub/Mul/BitAnd/BitOr/BitXor { src; dst }
//                            -> [ <op> src, dst ]
//   DivSigned { src; dst }   -> [ movl dst, %eax; cdq; idivl src;
//                                  movl %eax, dst ]
//   RemSigned { src; dst }   -> [ movl dst, %eax; cdq; idivl src;
//                                  movl %edx, dst ]
//   BitShiftLeft { src; dst }
//                            -> [ movl src, %ecx; sall %cl, dst ]
//   BitShiftRight { src; dst }
//                            -> [ movl src, %ecx; sarl %cl, dst ]
//
// The pseudoregisters survive the codegen pass and are resolved into
// `%rbp`-relative `Stack(offset)` operands by `replace_pseudos` once
// the function has walked its frame.  Frames stay empty for chapter 3
// because the only locals we allocate are temporaries; the parameter
// stays so the pipeline signature remains stable across waves.

use anyhow::Result;

use crate::codegen::assembly::{
    AsmProgram, BinaryOpInstr, Instr, Operand, Reg, TopLevel, UnaryOpInstr,
};
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
        Instruction::Add { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::Add,
            src: convert_val(src),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::Sub { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::Sub,
            src: convert_val(src),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::Mul { src, dst } => vec![
            Instr::Mov {
                src: Operand::Pseudo(dst.clone()),
                dst: Operand::Reg(Reg::AX),
            },
            Instr::Mov {
                src: convert_val(src),
                dst: Operand::Reg(Reg::R10),
            },
            Instr::BinaryOp {
                op: BinaryOpInstr::Mult,
                src: Operand::Reg(Reg::R10),
                dst: Operand::Reg(Reg::AX),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::AX),
                dst: Operand::Pseudo(dst.clone()),
            },
        ],
        Instruction::DivSigned { src, dst } => vec![
            Instr::Mov {
                src: Operand::Pseudo(dst.clone()),
                dst: Operand::Reg(Reg::AX),
            },
            Instr::Mov {
                src: convert_val(src),
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cdq,
            Instr::Idiv(Operand::Reg(Reg::R10)),
            Instr::Mov {
                src: Operand::Reg(Reg::AX),
                dst: Operand::Pseudo(dst.clone()),
            },
        ],
        Instruction::RemSigned { src, dst } => vec![
            Instr::Mov {
                src: Operand::Pseudo(dst.clone()),
                dst: Operand::Reg(Reg::AX),
            },
            Instr::Mov {
                src: convert_val(src),
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cdq,
            Instr::Idiv(Operand::Reg(Reg::R10)),
            Instr::Mov {
                src: Operand::Reg(Reg::DX),
                dst: Operand::Pseudo(dst.clone()),
            },
        ],
        Instruction::BitAnd { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::BitAnd,
            src: convert_val(src),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::BitOr { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::BitOr,
            src: convert_val(src),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::BitXor { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::BitXor,
            src: convert_val(src),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::BitShiftLeft { src, dst } => vec![
            Instr::Mov {
                src: convert_val(src),
                dst: Operand::Reg(Reg::CX),
            },
            Instr::BinaryOp {
                op: BinaryOpInstr::BitShiftLeft,
                src: Operand::Reg(Reg::CX),
                dst: Operand::Pseudo(dst.clone()),
            },
        ],
        Instruction::BitShiftRight { src, dst } => vec![
            Instr::Mov {
                src: convert_val(src),
                dst: Operand::Reg(Reg::CX),
            },
            Instr::BinaryOp {
                op: BinaryOpInstr::BitShiftRight,
                src: Operand::Reg(Reg::CX),
                dst: Operand::Pseudo(dst.clone()),
            },
        ],
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