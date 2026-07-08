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
// Chapter 9 adds:
//   - `Call { name, args, dst }` -> classify each arg via the
//     System V AMD64 ABI, push the integer args into %rdi..%r9 (or
//     push the rest on the stack), align the stack to 16 bytes,
//     emit `call name`, restore the stack pointer, and copy the
//     return value from %eax into `dst` if `dst` is `Some`.
//   - A per-function prologue that moves each parameter from its
//     incoming register into a function-local pseudo slot, so the
//     replace_pseudos pass can give each parameter its own stack
//     frame slot.
//   - Multi-function lowering: each `TackyFunction` becomes one
//     `TopLevel::Fn` entry; the body is the union of prologue and
//     per-instruction lowering.

use anyhow::Result;

use crate::codegen::abi::{self, ParamClass};
use crate::codegen::assembly::{
    AsmProgram, BinaryOpInstr, ConditionCode as AsmCC, Instr, Operand, Reg, StaticInit, TopLevel,
    UnaryOpInstr,
};
use crate::codegen::frame::Frame;
use crate::ir::tacky::{
    ConditionCode as TackyCC, Instruction, OperandType, TackyFunction, TackyProgram, TypeEnv, Val,
};

/// Look up the operand width of a TACKY value.  Constants default
/// to `Int` (the lowerer materialises long constants into a
/// synthetic `const.N` name and records the type in the env);
/// named variables consult the function's `type_env`.  Unknown
/// names default to `Int` so the chapter-1..10 codegen keeps
/// compiling unchanged.  Chapter 13: a `ConstantDouble` operand
/// has type `Double`.
fn type_of_val(val: &Val, env: &TypeEnv) -> OperandType {
    match val {
        Val::Constant(_) => OperandType::Int,
        Val::ConstantDouble(_) => OperandType::Double,
        Val::Var(name) => env.get(name).copied().unwrap_or(OperandType::Int),
    }
}

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
        Val::ConstantDouble(_) => {
            // Constant doubles are materialised into a constant-pool
            // entry by the codegen pass; the lowerer keeps the
            // `Val::ConstantDouble` form so type info survives, and
            // the codegen sees it just before the per-instruction
            // emit phase materialises the pool label.
            Operand::Pseudo(String::new()) // placeholder; never used
        }
        Val::Var(name) => Operand::Pseudo(name.clone()),
    }
}

/// Returns true when `n` does not fit in a sign-extended 32-bit
/// immediate.  x86-64 `movq imm32, mem` requires the immediate to
/// be representable as `i32`; values outside that range must be
/// loaded via `movabsq` into a register first.
fn immediate_too_wide(n: i64) -> bool {
    n < i32::MIN as i64 || n > i32::MAX as i64
}

/// Map an ABI register slot to the assembly `Reg` enum.  Mirrors the
/// first six entries of `INT_PARAM_REGS` in `codegen::abi`.
fn abi_reg(reg: abi::Reg) -> Reg {
    match reg {
        abi::Reg::DI => Reg::DI,
        abi::Reg::SI => Reg::SI,
        abi::Reg::DX => Reg::DX,
        abi::Reg::CX => Reg::CX,
        abi::Reg::R8 => Reg::R8,
        abi::Reg::R9 => Reg::R9,
    }
}

/// Lower a `Call { name, args, dst }` instruction.
///
/// Mirrors `convert_function_call` in
/// `nqcc2/lib/backend/codegen.ml:339-448`, simplified to the integer
/// case (no SSE doubles, no struct returns, no struct args).
///
/// Steps:
/// 1. Classify each argument via the ABI plan (first six in regs,
///    rest on the stack).
/// 2. Compute stack padding so the call site is 16-byte aligned
///    *after* the pushes for stack arguments.
/// 3. Emit `subq $pad, %rsp` if padding is needed.
/// 4. Emit one `movl arg, %rdi/%rsi/...` per register argument.
/// 5. Emit `pushq arg` per stack argument (in reverse source order
///    so the first argument ends up at the lowest stack address).
/// 6. Emit `call name`.
/// 7. Emit `addq $total, %rsp` to undo the pushes + padding.
/// 8. If `dst` is `Some`, emit `movl %eax, dst` so the call site
///    sees the return value in a pseudo slot.
fn lower_call(name: &str, args: &[Val], dst: &Option<String>, type_env: &TypeEnv) -> Vec<Instr> {
    let plan = abi::classify_params(args.len());
    let mut out: Vec<Instr> = Vec::new();

    // Compute stack padding: we need 16-byte alignment before the
    // `call` instruction.  On entry to a function call site the
    // stack is 8-byte aligned (after the `call` itself pushes an
    // 8-byte return address, the called function sees a 16-byte
    // aligned stack).  After we push N stack arguments (each 8
    // bytes), the alignment depends on whether N is even or odd:
    //   N=0: stack is 8-byte aligned, no padding needed.
    //   N=2: stack is 24-byte aligned (8 + 16), still 8 mod 16,
    //        need 8 bytes of padding so that after the call pushes
    //        the return address, the called function sees 16-byte
    //        alignment.
    //   N=1: stack is 16-byte aligned (8 + 8), need 0 bytes of
    //        padding — the next 8-byte slot is already 16 mod 16.
    // In general: pad by 8 when N is even.
    let stack_arg_count: usize = plan
        .param_classes
        .iter()
        .filter(|c| matches!(c, ParamClass::Stack))
        .count();
    let padding = if stack_arg_count.is_multiple_of(2) {
        0
    } else {
        8
    };
    if padding != 0 {
        out.push(Instr::AllocateStack(padding));
    }

    // Register-passed arguments: emit `mov arg, reg`.  Mirrors
    // `pass_int_reg_arg` in OCaml `convert_function_call:371-381`.
    // Chapter 11: pick `movl` vs `movq` from the argument's
    // type so a long argument isn't silently truncated to 32 bits.
    for (idx, val) in args.iter().enumerate() {
        if plan.param_classes[idx] == ParamClass::Int {
            if type_of_val(val, type_env) == OperandType::Long {
                out.push(Instr::Movq {
                    src: convert_val(val),
                    dst: Operand::Reg(abi_reg(abi::int_param_reg(idx))),
                });
            } else {
                out.push(Instr::Mov {
                    src: convert_val(val),
                    dst: Operand::Reg(abi_reg(abi::int_param_reg(idx))),
                });
            }
        }
    }

    // Stack-passed arguments: emit `push arg` in reverse source
    // order so that arg[6] (the first stack arg) ends up at the
    // lowest stack address.  Mirrors OCaml `pass_stack_arg` +
    // `List.rev_map` at the bottom of `convert_function_call`.
    let mut stack_instrs: Vec<Instr> = Vec::new();
    for (idx, val) in args.iter().enumerate().rev() {
        if plan.param_classes[idx] == ParamClass::Stack {
            stack_instrs.push(Instr::Push(convert_val(val)));
        }
    }
    out.extend(stack_instrs);

    // Emit the `call` itself.
    out.push(Instr::Call(name.to_string()));

    // Restore the stack pointer: total bytes removed = padding +
    // 8 * stack_arg_count.  Mirrors the `dealloc` block at OCaml
    // `convert_function_call:411-424`.
    let bytes_to_remove = padding + 8 * (stack_arg_count as i32);
    if bytes_to_remove != 0 {
        out.push(Instr::DeallocateStack(bytes_to_remove));
    }

    // Capture the return value if the call site expects one.
    // Chapter 11: always use `movq` to copy the full 64-bit
    // return register into the destination pseudo, so the upper
    // 32 bits aren't left as garbage when the caller only meant
    // to read the low 32 (e.g. `movl %eax, dst` only writes 4
    // bytes and leaves the high half of the slot undefined).
    if let Some(dst_name) = dst {
        out.push(Instr::Movq {
            src: Operand::Reg(Reg::AX),
            dst: Operand::Pseudo(dst_name.clone()),
        });
    }
    out
}

/// Lower a single TACKY instruction into a flat list of assembly
/// instructions.  Mirrors `convert_instruction` in
/// `nqcc2/lib/backend/codegen.ml:482-803` (integer subset).
///
/// `env` is the function's `type_env` — chapter 11 uses it to pick
/// between 32-bit (`movl`/`addl`/...) and 64-bit (`movq`/`addq`/...)
/// instruction variants for every operand-width-sensitive op.  When
/// the env records a variable as `Long`, the codegen emits the
/// quadword form; otherwise it emits the longword form.
fn lower_instruction(instr: &Instruction, env: &TypeEnv) -> Vec<Instr> {
    match instr {
        Instruction::Return(val) => {
            let is_long = type_of_val(val, env) == OperandType::Long;
            let mut out: Vec<Instr> = Vec::new();
            if is_long {
                // A 64-bit immediate that doesn't fit in i32 needs
                // `movabsq` to %rax (since `movq imm32, %rax` is
                // the only form that takes a memory-ish immediate).
                if let Val::Constant(n) = val {
                    if immediate_too_wide(*n) {
                        out.push(Instr::Movabsq {
                            src: *n,
                            dst: Operand::Reg(Reg::AX),
                        });
                        out.push(Instr::Ret);
                        return out;
                    }
                }
                out.push(Instr::Movq {
                    src: convert_val(val),
                    dst: Operand::Reg(Reg::AX),
                });
            } else {
                out.push(Instr::Mov {
                    src: convert_val(val),
                    dst: Operand::Reg(Reg::AX),
                });
            }
            out.push(Instr::Ret);
            out
        }
        Instruction::Copy { src, dst } => {
            // Copying into a `long` slot moves 64 bits even if the
            // source is a narrow int.  The destination's type wins.
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty == OperandType::Long {
                // 64-bit immediates outside the i32 range need
                // `movabsq` into a register first; `movq imm32, mem`
                // is the only direct immediate form.
                if let Val::Constant(n) = src {
                    if immediate_too_wide(*n) {
                        return vec![
                            Instr::Movabsq {
                                src: *n,
                                dst: Operand::Reg(Reg::R10),
                            },
                            Instr::Movq {
                                src: Operand::Reg(Reg::R10),
                                dst: Operand::Pseudo(dst.clone()),
                            },
                        ];
                    }
                }
                vec![Instr::Movq {
                    src: convert_val(src),
                    dst: Operand::Pseudo(dst.clone()),
                }]
            } else {
                vec![Instr::Mov {
                    src: convert_val(src),
                    dst: Operand::Pseudo(dst.clone()),
                }]
            }
        }
        Instruction::SignExtend { src, dst } => {
            // `movslq $imm, %reg` is rejected by some assemblers
            // ("operand type mismatch"); route immediates through a
            // 32-bit register first so the assembler sees a legal
            // source operand.  `movslq` also requires a register
            // destination, so we stage through `%r10` and then move
            // the result into the destination pseudo.  Constants that
            // don't fit in a signed 32-bit value (e.g. `2147483653L`)
            // need `movabsq` so the 64-bit immediate isn't silently
            // truncated by `movl`.
            let src_op = convert_val(src);
            match src_op {
                Operand::Imm(n) => {
                    if n >= i64::from(i32::MIN) && n <= i64::from(i32::MAX) {
                        vec![
                            Instr::Mov {
                                src: Operand::Imm(n),
                                dst: Operand::Reg(Reg::R10),
                            },
                            Instr::Movsx {
                                src: Operand::Reg(Reg::R10),
                                dst: Operand::Reg(Reg::R10),
                            },
                            Instr::Movq {
                                src: Operand::Reg(Reg::R10),
                                dst: Operand::Pseudo(dst.clone()),
                            },
                        ]
                    } else {
                        vec![
                            Instr::Movabsq {
                                src: n,
                                dst: Operand::Reg(Reg::R10),
                            },
                            Instr::Movq {
                                src: Operand::Reg(Reg::R10),
                                dst: Operand::Pseudo(dst.clone()),
                            },
                        ]
                    }
                }
                _ => vec![Instr::Movsx {
                    src: src_op,
                    dst: Operand::Pseudo(dst.clone()),
                }],
            }
        }
        // Truncate a `long` value to `int` by routing through a
        // 32-bit move.  `movq src, %r10d` reads the low 32 bits
        // into %r10d (zero-extends in x86-64); `movl %r10d, dst`
        // writes the truncated value.  For pointers (still 64-bit)
        // this lowers to a plain `movq`; for plain `long` it
        // narrows to 32 bits.
        Instruction::Truncate { src, dst } => vec![
            Instr::Movq {
                src: convert_val(src),
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Mov {
                src: Operand::Reg(Reg::R10),
                dst: Operand::Pseudo(dst.clone()),
            },
        ],
        Instruction::Negate { dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            vec![if is_long {
                Instr::UnaryQ {
                    op: UnaryOpInstr::Neg,
                    operand: Operand::Pseudo(dst.clone()),
                }
            } else {
                Instr::Unary {
                    op: UnaryOpInstr::Neg,
                    operand: Operand::Pseudo(dst.clone()),
                }
            }]
        }
        Instruction::Complement { dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            vec![if is_long {
                Instr::UnaryQ {
                    op: UnaryOpInstr::Not,
                    operand: Operand::Pseudo(dst.clone()),
                }
            } else {
                Instr::Unary {
                    op: UnaryOpInstr::Not,
                    operand: Operand::Pseudo(dst.clone()),
                }
            }]
        }
        Instruction::Add { src, dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            let op = if is_long {
                BinaryOpInstr::AddQ
            } else {
                BinaryOpInstr::Add
            };
            vec![Instr::BinaryOp {
                op,
                src: convert_val(src),
                dst: Operand::Pseudo(dst.clone()),
            }]
        }
        Instruction::Sub { src, dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            let op = if is_long {
                BinaryOpInstr::SubQ
            } else {
                BinaryOpInstr::Sub
            };
            vec![Instr::BinaryOp {
                op,
                src: convert_val(src),
                dst: Operand::Pseudo(dst.clone()),
            }]
        }
        Instruction::Mul { src, dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            let op = if is_long {
                BinaryOpInstr::MultQ
            } else {
                BinaryOpInstr::Mult
            };
            let (mov_to_ax, mov_to_r10, mov_from_ax) = if is_long {
                (
                    Instr::Movq {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::AX),
                    },
                    Instr::Movq {
                        src: convert_val(src),
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Movq {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                )
            } else {
                (
                    Instr::Mov {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::AX),
                    },
                    Instr::Mov {
                        src: convert_val(src),
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Mov {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                )
            };
            vec![
                mov_to_ax,
                mov_to_r10,
                Instr::BinaryOp {
                    op,
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Reg(Reg::AX),
                },
                mov_from_ax,
            ]
        }
        Instruction::DivSigned { src, dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            if is_long {
                vec![
                    Instr::Movq {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::AX),
                    },
                    Instr::Movq {
                        src: convert_val(src),
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cqo,
                    Instr::Idivq(Operand::Reg(Reg::R10)),
                    Instr::Movq {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ]
            } else {
                vec![
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
                ]
            }
        }
        Instruction::RemSigned { src, dst } => {
            let is_long = type_of_val(&Val::Var(dst.clone()), env) == OperandType::Long;
            if is_long {
                vec![
                    Instr::Movq {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::AX),
                    },
                    Instr::Movq {
                        src: convert_val(src),
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cqo,
                    Instr::Idivq(Operand::Reg(Reg::R10)),
                    Instr::Movq {
                        src: Operand::Reg(Reg::DX),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ]
            } else {
                vec![
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
                ]
            }
        }
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
        //
        // Chapter 11: when the operands are `long`, emit `cmpq`
        // (64-bit compare) instead of `cmpl`.  The `SetCC` /
        // `MovZeroExtend` tail still produces a 32-bit int (the
        // comparison result is 0 or 1, regardless of operand width).
        Instruction::Cmp { left, right, dst, cc } => {
            let left_op = convert_val(left);
            let right_op = convert_val(right);
            let is_long = type_of_val(left, env) == OperandType::Long
                || type_of_val(right, env) == OperandType::Long;
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
            let scratch_reg = Reg::R11;
            let (prelude, cmp_left) = match &left_op {
                Operand::Imm(_) => {
                    let mov = if is_long {
                        Instr::Movq {
                            src: left_op,
                            dst: Operand::Reg(scratch_reg.clone()),
                        }
                    } else {
                        Instr::Mov {
                            src: left_op,
                            dst: Operand::Reg(scratch_reg.clone()),
                        }
                    };
                    (vec![mov], Operand::Reg(scratch_reg))
                }
                _ => (Vec::new(), left_op),
            };
            let mut out = prelude;
            if is_long {
                out.push(Instr::Cmpq {
                    left: cmp_left,
                    right: right_op,
                });
            } else {
                out.push(Instr::Cmp {
                    left: cmp_left,
                    right: right_op,
                });
            }
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
        //
        // Chapter 11: when `condition` is `long`, use `cmpq $0, cond`
        // (64-bit compare) instead of `cmpl $0, cond`.
        Instruction::JumpIfZero { condition, target } => {
            let cond_op = convert_val(condition);
            let is_long = type_of_val(condition, env) == OperandType::Long;
            let (prelude, cmp_left) = match &cond_op {
                Operand::Imm(_) => {
                    let mov = if is_long {
                        Instr::Movq {
                            src: cond_op,
                            dst: Operand::Reg(Reg::R10),
                        }
                    } else {
                        Instr::Mov {
                            src: cond_op,
                            dst: Operand::Reg(Reg::R10),
                        }
                    };
                    (vec![mov], Operand::Reg(Reg::R10))
                }
                _ => (Vec::new(), cond_op),
            };
            let mut out = prelude;
            if is_long {
                out.push(Instr::Cmpq {
                    left: cmp_left,
                    right: Operand::Imm(0),
                });
            } else {
                out.push(Instr::Cmp {
                    left: cmp_left,
                    right: Operand::Imm(0),
                });
            }
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
        // Chapter 11: when `condition` is `long`, use `cmpq`.
        Instruction::JumpIfNotZero { condition, target } => {
            let cond_op = convert_val(condition);
            let is_long = type_of_val(condition, env) == OperandType::Long;
            let (prelude, cmp_left) = match &cond_op {
                Operand::Imm(_) => {
                    let mov = if is_long {
                        Instr::Movq {
                            src: cond_op,
                            dst: Operand::Reg(Reg::R10),
                        }
                    } else {
                        Instr::Mov {
                            src: cond_op,
                            dst: Operand::Reg(Reg::R10),
                        }
                    };
                    (vec![mov], Operand::Reg(Reg::R10))
                }
                _ => (Vec::new(), cond_op),
            };
            let mut out = prelude;
            if is_long {
                out.push(Instr::Cmpq {
                    left: cmp_left,
                    right: Operand::Imm(0),
                });
            } else {
                out.push(Instr::Cmp {
                    left: cmp_left,
                    right: Operand::Imm(0),
                });
            }
            out.push(Instr::JmpCC {
                cc: AsmCC::NE,
                label: target.clone(),
            });
            out
        },
        // Chapter 4 short-circuit `&&` / `||` lowering materialises
        // forward labels for the join point.
        Instruction::Label(name) => vec![Instr::Label(name.clone())],
        Instruction::Call { name, args, dst } => lower_call(name, args, dst, env),
        _ => Vec::new(),
    }
}

/// Lower a single function definition into a `TopLevel::Fn`.
///
/// Mirrors `convert_top_level` in `nqcc2/lib/backend/codegen.ml:858-866`
/// for the `Function` branch, plus the prologue emitted by the
/// book's chapter-9 walk (`pass_params` at
/// `nqcc2/lib/backend/codegen.ml:805-837`).
///
/// The prologue emits one `movl %rdi, param.0`-style move per
/// parameter so the rest of the function can refer to the parameter
/// by its stack slot (resolved by `replace_pseudos`).  Parameters 7+
/// are copied from the caller-passed stack slot (`16(%rbp)`, `24(%rbp)`,
/// ...) by `replace_pseudos` once the frame is laid out.
fn generate_function(func: &TackyFunction, globals: &TypeEnv) -> TopLevel {
    let plan = abi::classify_params(func.params.len());
    let mut prologue: Vec<Instr> = Vec::new();
    for (idx, param_name) in func.params.iter().enumerate() {
        // Chapter 11: long parameters are passed in the same
        // integer registers but occupy 8 bytes.  Emit a `Movq`
        // for them so the stack slot is sized correctly.
        let is_long = func
            .type_env
            .get(param_name)
            .copied()
            .unwrap_or(OperandType::Int)
            == OperandType::Long;
        match plan.param_classes[idx] {
            ParamClass::Int => {
                let src = Operand::Reg(abi_reg(abi::int_param_reg(idx)));
                let dst = Operand::Pseudo(param_name.clone());
                if is_long {
                    prologue.push(Instr::Movq { src, dst });
                } else {
                    prologue.push(Instr::Mov { src, dst });
                }
            }
            ParamClass::Stack => {
                // Caller-passed argument at `16(%rbp) + 8*(idx-6)`.
                // We emit a `Mov` from a `Memory(BP, offset)` operand;
                // `replace_pseudos` resolves the source `Memory`
                // operand into the actual stack location once the
                // frame is laid out.  The destination is a pseudo so
                // the body can reference the parameter by name.
                let offset = 16 + (8 * (idx - 6)) as i32;
                let src = Operand::Memory(Reg::BP, offset);
                let dst = Operand::Pseudo(param_name.clone());
                if is_long {
                    prologue.push(Instr::Movq { src, dst });
                } else {
                    prologue.push(Instr::Mov { src, dst });
                }
            }
            ParamClass::Sse => {
                // Chapter 13; not yet emitted in chapter 9.
            }
        }
    }
    // Merge the function's local type env with the file-scope
    // global type map so a `Copy` of a `long` global reads it
    // with `movq` (not `movl`).  The function's own type_env
    // already has every local / parameter / materialised long
    // constant; we layer the globals underneath so locals win.
    let mut merged = globals.clone();
    for (k, v) in &func.type_env {
        merged.insert(k.clone(), *v);
    }
    let body = func
        .body
        .iter()
        .flat_map(|instr| lower_instruction(instr, &merged))
        .collect::<Vec<_>>();
    let mut instructions = Vec::with_capacity(prologue.len() + body.len());
    instructions.extend(prologue);
    instructions.extend(body);
    TopLevel::Fn {
        name: func.name.clone(),
        global: func.global,
        instructions,
        type_env: func.type_env.clone(),
    }
}

pub fn generate(tacky: &TackyProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    let mut top_level: Vec<TopLevel> = Vec::new();
    for var in &tacky.static_variables {
        let init = match (var.init.clone(), var.ty) {
            (crate::ir::tacky::TackyStaticInit::Int(n), OperandType::Long) => StaticInit::Long(n),
            (crate::ir::tacky::TackyStaticInit::Int(n), _) => StaticInit::Int(n),
            (crate::ir::tacky::TackyStaticInit::Zero, OperandType::Long) => StaticInit::Zero(8),
            (crate::ir::tacky::TackyStaticInit::Zero, OperandType::Double) => StaticInit::Zero(8),
            (crate::ir::tacky::TackyStaticInit::Zero, _) => StaticInit::Zero(4),
            (crate::ir::tacky::TackyStaticInit::Long(n), _) => StaticInit::Long(n),
            (crate::ir::tacky::TackyStaticInit::Double(d), _) => StaticInit::Double(d),
        };
        let alignment = match var.ty {
            OperandType::Long | OperandType::ULong | OperandType::Double => 8,
            _ => 4,
        };
        top_level.push(TopLevel::StaticVariable {
            name: var.name.clone(),
            global: var.global,
            alignment,
            init,
        });
    }
    // Build a name -> type map for the file-scope statics so
    // every function's codegen can pick the right width when
    // copying a global into a local.  Globals are layered
    // beneath the function's own type_env so locals win.
    let mut globals_type_env: TypeEnv = TypeEnv::new();
    for var in &tacky.static_variables {
        globals_type_env.insert(var.name.clone(), var.ty);
    }
    for func in &tacky.functions {
        top_level.push(generate_function(func, &globals_type_env));
    }
    Ok(AsmProgram { top_level })
}