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

use crate::codegen::assembly::{AsmProgram, BinaryOpInstr, Instr, Operand, Reg, TopLevel};

/// Split one two-operand x86-64 form into a register-routed pair.
/// Returns a `Vec` because most rules emit exactly one move + one
/// op, but a few emit two moves (for the double-register case).
fn split_mem_to_mem(instr: Instr) -> Vec<Instr> {
    match instr {
        // `movl mem, mem` is invalid — route through %r10.
        Instr::Mov {
            src: src @ (Operand::Memory(..) | Operand::Stack(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        // Chapter 11: same split for `movq mem, mem`.
        Instr::Movq {
            src: src @ (Operand::Memory(..) | Operand::Stack(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Movsd {
            src: src @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Movsd {
                src,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::Movsd {
                src: Operand::Reg(Reg::XMM(15)),
                dst,
            },
        ],
        Instr::BinaryOp {
            op: op @ (BinaryOpInstr::Mult | BinaryOpInstr::MultQ),
            src,
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => {
            let scratch_reg = if src == Operand::Reg(Reg::R11) {
                Reg::R10
            } else {
                Reg::R11
            };
            let scratch = Operand::Reg(scratch_reg.clone());
            let load = if matches!(op, BinaryOpInstr::MultQ) {
                Instr::Movq {
                    src: dst.clone(),
                    dst: scratch.clone(),
                }
            } else {
                Instr::Mov {
                    src: dst.clone(),
                    dst: scratch.clone(),
                }
            };
            let store = if matches!(op, BinaryOpInstr::MultQ) {
                Instr::Movq {
                    src: scratch.clone(),
                    dst,
                }
            } else {
                Instr::Mov {
                    src: scratch.clone(),
                    dst,
                }
            };
            let mut out = Vec::new();
            if scratch_reg == Reg::R11 {
                out.push(Instr::Push(Operand::Reg(Reg::R11)));
            }
            out.extend([
                load,
                Instr::BinaryOp {
                    op,
                    src,
                    dst: scratch,
                },
                store,
            ]);
            if scratch_reg == Reg::R11 {
                out.push(Instr::Pop(Reg::R11));
            }
            out
        }
        // `binaryOp op mem, mem` is invalid — route through %r10.
        // Chapter 11: the 64-bit ops (AddQ/SubQ/MultQ/DivQ/RemQ)
        // require a 64-bit scratch move, not the default 32-bit.
        Instr::BinaryOp {
            op,
            src: src @ (Operand::Memory(..) | Operand::Stack(_)),
            dst: dst @ (Operand::Memory(..) | Operand::Stack(_)),
        } => {
            let is_wide = matches!(
                op,
                BinaryOpInstr::AddQ
                    | BinaryOpInstr::SubQ
                    | BinaryOpInstr::MultQ
                    | BinaryOpInstr::DivQ
                    | BinaryOpInstr::RemQ
                    | BinaryOpInstr::BitAndQ
                    | BinaryOpInstr::BitOrQ
            );
            let (pre_mov, post_op) = if is_wide {
                (
                    Instr::Movq {
                        src,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::BinaryOp {
                        op,
                        src: Operand::Reg(Reg::R10),
                        dst,
                    },
                )
            } else {
                (
                    Instr::Mov {
                        src,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::BinaryOp {
                        op,
                        src: Operand::Reg(Reg::R10),
                        dst,
                    },
                )
            };
            vec![pre_mov, post_op]
        }
        // `cmpl mem, mem` is invalid — route through %r10.
        Instr::Cmp {
            left,
            right: right @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Mov {
                src: right,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cmp {
                left,
                right: Operand::Reg(Reg::R10),
            },
        ],
        // Chapter 11: same split for `cmpq mem, mem`.
        Instr::Cmpq {
            left,
            right: right @ (Operand::Memory(..) | Operand::Stack(_)),
        } => vec![
            Instr::Movq {
                src: right,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cmpq {
                left,
                right: Operand::Reg(Reg::R10),
            },
        ],
        Instr::CmpDouble {
            left,
            right: right @ (Operand::Memory(..) | Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Movsd {
                src: right,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::CmpDouble {
                left,
                right: Operand::Reg(Reg::XMM(15)),
            },
        ],
        // `idivl mem` is invalid — route through %r10.
        Instr::Idiv(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Idiv(Operand::Reg(Reg::R10)),
        ],
        Instr::Div(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Div(Operand::Reg(Reg::R10)),
        ],
        // Chapter 11: same split for `idivq mem`.
        Instr::Idivq(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Idivq(Operand::Reg(Reg::R10)),
        ],
        Instr::Divq(src @ (Operand::Memory(..) | Operand::Stack(_))) => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Divq(Operand::Reg(Reg::R10)),
        ],
        // Anything else passes through unchanged.
        other => vec![other],
    }
}

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
