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
use crate::ir::tacky::{ConditionCode as TackyCC, Instruction, TackyFunction, TackyProgram, Val};

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
fn lower_call(name: &str, args: &[Val], dst: &Option<String>) -> Vec<Instr> {
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
    for (idx, val) in args.iter().enumerate() {
        if plan.param_classes[idx] == ParamClass::Int {
            out.push(Instr::Mov {
                src: convert_val(val),
                dst: Operand::Reg(abi_reg(abi::int_param_reg(idx))),
            });
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
    if let Some(dst_name) = dst {
        out.push(Instr::Mov {
            src: Operand::Reg(Reg::AX),
            dst: Operand::Pseudo(dst_name.clone()),
        });
    }
    out
}

/// Lower a single TACKY instruction into a flat list of assembly
/// instructions.  Mirrors `convert_instruction` in
/// `nqcc2/lib/backend/codegen.ml:482-803` (integer subset).
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
        Instruction::Call { name, args, dst } => lower_call(name, args, dst),
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
fn generate_function(func: &TackyFunction) -> TopLevel {
    let plan = abi::classify_params(func.params.len());
    let mut prologue: Vec<Instr> = Vec::new();
    for (idx, param_name) in func.params.iter().enumerate() {
        match plan.param_classes[idx] {
            ParamClass::Int => {
                prologue.push(Instr::Mov {
                    src: Operand::Reg(abi_reg(abi::int_param_reg(idx))),
                    dst: Operand::Pseudo(param_name.clone()),
                });
            }
            ParamClass::Stack => {
                // Caller-passed argument at `16(%rbp) + 8*(idx-6)`.
                // We emit a `Mov` from a `Memory(BP, offset)` operand;
                // `replace_pseudos` resolves the source `Memory`
                // operand into the actual stack location once the
                // frame is laid out.  The destination is a pseudo so
                // the body can reference the parameter by name.
                let offset = 16 + (8 * (idx - 6)) as i32;
                prologue.push(Instr::Mov {
                    src: Operand::Memory(Reg::BP, offset),
                    dst: Operand::Pseudo(param_name.clone()),
                });
            }
            ParamClass::Sse => {
                // Chapter 13; not yet emitted in chapter 9.
            }
        }
    }
    let body = func
        .body
        .iter()
        .flat_map(lower_instruction)
        .collect::<Vec<_>>();
    let mut instructions = Vec::with_capacity(prologue.len() + body.len());
    instructions.extend(prologue);
    instructions.extend(body);
    TopLevel::Fn {
        name: func.name.clone(),
        global: func.global,
        instructions,
    }
}

pub fn generate(tacky: &TackyProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    let mut top_level: Vec<TopLevel> = Vec::new();
    for var in &tacky.static_variables {
        let init = match var.init {
            crate::ir::tacky::TackyStaticInit::Int(n) => StaticInit::Int(n),
            crate::ir::tacky::TackyStaticInit::Zero => StaticInit::Zero(4),
        };
        top_level.push(TopLevel::StaticVariable {
            name: var.name.clone(),
            global: var.global,
            alignment: 4,
            init,
        });
    }
    for func in &tacky.functions {
        top_level.push(generate_function(func));
    }
    Ok(AsmProgram { top_level })
}