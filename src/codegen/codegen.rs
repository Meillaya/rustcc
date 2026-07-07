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
    AsmProgram, BinaryOpInstr, ConditionCode as AsmCC, Instr, Operand, Reg, TopLevel, UnaryOpInstr,
};
use crate::codegen::frame::Frame;
use crate::ir::tacky::{ConditionCode as TackyCC, Instruction, TackyProgram, Val};

/// Convert a TACKY [`ConditionCode`] to the equivalent assembly
/// [`assembly::ConditionCode`].  The two enums carry the same set of
/// variants but live in different modules to keep the IR layer free
/// of codegen dependencies; the conversion is structural and trivial.
fn map_cc(cc: TackyCC) -> AsmCC {
    match cc {
        TackyCC::E => AsmCC::E,
        TackyCC::NE => AsmCC::NE,
        TackyCC::L => AsmCC::L,
        TackyCC::LE => AsmCC::LE,
        TackyCC::G => AsmCC::G,
        TackyCC::GE => AsmCC::GE,
        TackyCC::A => AsmCC::A,
        TackyCC::AE => AsmCC::AE,
        TackyCC::B => AsmCC::B,
        TackyCC::BE => AsmCC::BE,
        TackyCC::P => AsmCC::P,
    }
}

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
        // Chapter 4 comparison: emit
        //   cmpl right, left      ; flags = left - right
        //   setCC cc dst           ; 0/1 byte to dst low byte
        //   movzbl dst, dst        ; zero-extend byte to 32-bit int
        // so the destination holds a normalized 0 or 1.  The OCaml
        // reference emits an extra `mov $0, dst` before `setCC` to
        // defensively clear the destination; in the Rust port
        // `movzbl` zero-extends the byte written by `setCC`, so the
        // clear is unnecessary.
        Instruction::Cmp { left, right, dst, cc } => {
            // x86-64 `cmp` requires its destination (the second AT&T
            // operand, which is the `left` operand in our IR) to be
            // a register or memory — never an immediate.  When the
            // lowerer forwarded an `Expr::Constant` for the left side,
            // route it through a scratch register before the
            // comparison so the emitted instruction is assembler-valid.
            //
            // Use `%r11d` (not `%r10d`) for this routing: the
            // `replace_pseudos` pass also uses `%r10d` when splitting
            // memory-to-memory `cmpl` operands, and reusing it here
            // would let the split clobber the value we just set up.
            let left_op = convert_val(left);
            let right_op = convert_val(right);
            let (prelude, cmp_left) = match &left_op {
                Operand::Imm(_) => (
                    vec![Instr::Mov {
                        src: left_op,
                        dst: Operand::Reg(Reg::R11),
                    }],
                    Operand::Reg(Reg::R11),
                ),
                _ => (Vec::new(), left_op),
            };
            let mut out = prelude;
            out.push(Instr::Cmp {
                left: cmp_left,
                right: right_op,
            });
            out.push(Instr::SetCC {
                cc: map_cc(*cc),
                dst: Operand::Pseudo(dst.clone()),
            });
            // `sete` only writes the destination's low byte; the upper
            // bytes of the destination are undefined.  Zero-extend the
            // byte through a scratch register, then write the full
            // 32-bit value back.  `movzbl` always reads its source as
            // a single byte, so the source must be the byte that
            // `sete` wrote (the destination), not a 32-bit register
            // copy of it.  Uses `%r10d` for the scratch register;
            // safe because no other codegen path uses `%r10d`
            // immediately after this sequence.
            out.push(Instr::MovZeroExtend {
                src: Operand::Pseudo(dst.clone()),
                dst: Operand::Reg(Reg::R10),
            });
            out.push(Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst: Operand::Pseudo(dst.clone()),
            });
            out
        }
        // Chapter 4 short-circuit `&&` / `||` lowering materialises
        // unconditional jumps.  Mirrors the OCaml `Jump` arm.
        Instruction::Jump { target } => vec![Instr::Jmp(target.clone())],
        // Chapter 4 short-circuit `&&` lowering materialises
        // `JumpIfZero`.  Codegen turns this into
        //   cmpl $0, cond
        //   je   target
        // When `cond` is itself an immediate (e.g. the lowerer
        // forwarded an `Expr::Constant` directly), x86-64 forbids
        // `cmpl imm, imm`; route through a scratch register so the
        // emitted instruction is assembler-valid.
        Instruction::JumpIfZero { condition, target } => {
            let cond_op = convert_val(condition);
            let (prelude, cmp_left) = match &cond_op {
                Operand::Imm(_) => (
                    vec![Instr::Mov {
                        src: cond_op,
                        dst: Operand::Reg(Reg::R10),
                    }],
                    Operand::Reg(Reg::R10),
                ),
                _ => (Vec::new(), cond_op),
            };
            let mut out = prelude;
            out.push(Instr::Cmp {
                left: cmp_left,
                right: Operand::Imm(0),
            });
            out.push(Instr::JmpCC {
                cc: AsmCC::E,
                label: target.clone(),
            });
            out
        },
        // Chapter 4 short-circuit `||` lowering materialises
        // `JumpIfNotZero`.  Codegen turns this into
        //   cmpl $0, cond
        //   jne  target
        // Same immediate-handling workaround as `JumpIfZero` above.
        Instruction::JumpIfNotZero { condition, target } => {
            let cond_op = convert_val(condition);
            let (prelude, cmp_left) = match &cond_op {
                Operand::Imm(_) => (
                    vec![Instr::Mov {
                        src: cond_op,
                        dst: Operand::Reg(Reg::R10),
                    }],
                    Operand::Reg(Reg::R10),
                ),
                _ => (Vec::new(), cond_op),
            };
            let mut out = prelude;
            out.push(Instr::Cmp {
                left: cmp_left,
                right: Operand::Imm(0),
            });
            out.push(Instr::JmpCC {
                cc: AsmCC::NE,
                label: target.clone(),
            });
            out
        },
        // Chapter 4 short-circuit `&&` / `||` lowering materialises
        // forward labels for the join point.
        Instruction::Label(name) => vec![Instr::Label(name.clone())],
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