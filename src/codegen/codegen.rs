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

use std::collections::{BTreeMap, BTreeSet, HashMap};

mod copy_prop_support;

use anyhow::Result;

use crate::ast::Type;
use crate::codegen::abi;
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

fn is_byte_type(ty: OperandType) -> bool {
    matches!(ty, OperandType::Byte | OperandType::UByte)
}

fn operand_type_for_scalar_return(ty: &Type) -> OperandType {
    match ty {
        Type::Char | Type::SignedChar => OperandType::Byte,
        Type::UnsignedChar => OperandType::UByte,
        Type::Long => OperandType::Long,
        Type::UnsignedLong | Type::Pointer(_) => OperandType::ULong,
        Type::Double => OperandType::Double,
        Type::UnsignedInt => OperandType::UInt,
        Type::Int => OperandType::Int,
        Type::Void | Type::Array { .. } | Type::Struct(_) | Type::Union(_) => OperandType::Int,
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

fn map_integer_cc(cc: TackyCC, is_unsigned: bool) -> AsmCC {
    if !is_unsigned {
        return map_cc(cc);
    }
    match cc {
        TackyCC::L => AsmCC::B,
        TackyCC::LE => AsmCC::BE,
        TackyCC::G => AsmCC::A,
        TackyCC::GE => AsmCC::AE,
        other => map_cc(other),
    }
}

fn double_cc(cc: TackyCC) -> AsmCC {
    match cc {
        TackyCC::L => AsmCC::B,
        TackyCC::LE => AsmCC::BE,
        TackyCC::G => AsmCC::A,
        TackyCC::GE => AsmCC::AE,
        other => map_cc(other),
    }
}

fn set_double_comparison(cc: TackyCC, dst: &str, ctx: &mut CodegenCtx) -> Vec<Instr> {
    let asm_cc = double_cc(cc);
    match cc {
        // `ucomisd` marks unordered comparisons with PF=1, CF=1, ZF=1.
        // That makes the raw equality, below, and below-or-equal condition
        // codes accept NaN operands.  Branch around the raw `setcc` so C's
        // NaN comparisons remain false for ==, <, and <=.
        TackyCC::E | TackyCC::L | TackyCC::LE => {
            let unordered = ctx.fresh_label("double_cmp.unordered_false");
            let end = ctx.fresh_label("double_cmp.end");
            vec![
                Instr::JmpCC {
                    cc: AsmCC::P,
                    label: unordered.clone(),
                },
                Instr::SetCC {
                    cc: asm_cc,
                    dst: Operand::Pseudo(dst.to_string()),
                },
                Instr::Jmp(end.clone()),
                Instr::Label(unordered),
                Instr::Mov {
                    src: Operand::Imm(0),
                    dst: Operand::Pseudo(dst.to_string()),
                },
                Instr::Label(end),
            ]
        }
        // C's `!=` is true for unordered comparisons.
        TackyCC::NE => {
            let unordered = ctx.fresh_label("double_cmp.unordered_true");
            let end = ctx.fresh_label("double_cmp.end");
            vec![
                Instr::JmpCC {
                    cc: AsmCC::P,
                    label: unordered.clone(),
                },
                Instr::SetCC {
                    cc: asm_cc,
                    dst: Operand::Pseudo(dst.to_string()),
                },
                Instr::Jmp(end.clone()),
                Instr::Label(unordered),
                Instr::Mov {
                    src: Operand::Imm(1),
                    dst: Operand::Pseudo(dst.to_string()),
                },
                Instr::Label(end),
            ]
        }
        // `seta` and `setae` are already false when PF=1 because unordered
        // also sets CF=1, so > and >= need no extra parity guard.
        TackyCC::G | TackyCC::GE => vec![Instr::SetCC {
            cc: asm_cc,
            dst: Operand::Pseudo(dst.to_string()),
        }],
        // The lowerer should only request signed comparison codes for double
        // expressions; keep the mapping total for robustness.
        _ => vec![Instr::SetCC {
            cc: asm_cc,
            dst: Operand::Pseudo(dst.to_string()),
        }],
    }
}

#[derive(Default)]
struct CodegenCtx {
    double_labels: HashMap<u64, String>,
    double_constants: Vec<(String, f64)>,
    label_counter: u32,
    function_param_types: HashMap<String, Vec<Type>>,
    function_return_types: HashMap<String, Type>,
    current_return_on_stack: bool,
    current_return_type: Option<Type>,
    current_function_name: String,
}

impl CodegenCtx {
    fn current_return_name(&self) -> &str {
        &self.current_function_name
    }
    fn double_label(&mut self, value: f64) -> String {
        let bits = value.to_bits();
        if let Some(label) = self.double_labels.get(&bits) {
            return label.clone();
        }
        let label = format!("dbl.{}", self.double_labels.len());
        self.double_labels.insert(bits, label.clone());
        self.double_constants.push((label.clone(), value));
        label
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let id = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}.{id}")
    }
}

fn convert_val(val: &Val, ctx: &mut CodegenCtx) -> Operand {
    match val {
        Val::Constant(n) => Operand::Imm(*n),
        Val::ConstantDouble(d) => Operand::Data(ctx.double_label(*d)),
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

fn slot_operand(val: &Val, offset: i64, ctx: &mut CodegenCtx) -> Operand {
    match val {
        Val::Var(name) => Operand::PseudoMem(name.clone(), offset as i32),
        _ => convert_val(val, ctx),
    }
}

fn copy_mem_to_reg(src: Operand, size: i64, reg: Reg) -> Vec<Instr> {
    match size {
        8 => vec![Instr::Movq {
            src,
            dst: Operand::Reg(reg),
        }],
        4 => vec![Instr::Mov {
            src,
            dst: Operand::Reg(reg),
        }],
        1 => vec![Instr::MovZeroExtend {
            src,
            dst: Operand::Reg(reg),
        }],
        _ => {
            let mut out = vec![
                Instr::AllocateStack(8),
                Instr::Movq {
                    src: Operand::Imm(0),
                    dst: Operand::Memory(Reg::SP, 0),
                },
            ];
            for offset in 0..size {
                out.push(Instr::MovByte {
                    src: add_offset(src.clone(), offset),
                    dst: Operand::Reg(Reg::R10),
                });
                out.push(Instr::MovByte {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Memory(Reg::SP, offset as i32),
                });
            }
            out.push(Instr::Pop(reg));
            out
        }
    }
}

fn copy_reg_to_mem(reg: Reg, dst: Operand, size: i64) -> Vec<Instr> {
    match size {
        8 => vec![Instr::Movq {
            src: Operand::Reg(reg),
            dst,
        }],
        4 => vec![Instr::Mov {
            src: Operand::Reg(reg),
            dst,
        }],
        1 => vec![Instr::MovByte {
            src: Operand::Reg(reg),
            dst,
        }],
        _ => {
            let mut out = vec![Instr::Push(Operand::Reg(reg.clone()))];
            for offset in 0..size {
                out.push(Instr::MovByte {
                    src: Operand::Memory(Reg::SP, offset as i32),
                    dst: add_offset(dst.clone(), offset),
                });
            }
            out.push(Instr::DeallocateStack(8));
            out
        }
    }
}

fn add_offset(op: Operand, offset: i64) -> Operand {
    match op {
        Operand::PseudoMem(name, base) => Operand::PseudoMem(name, base + offset as i32),
        Operand::Stack(base) => Operand::Stack(base + offset as i32),
        Operand::Data(name) => Operand::DataOffset(name, offset as i32),
        Operand::DataOffset(name, base) => Operand::DataOffset(name, base + offset as i32),
        Operand::Memory(reg, base) => Operand::Memory(reg, base + offset as i32),
        other => other,
    }
}

fn copy_mem_to_stack(src: Operand, size: i64) -> Vec<Instr> {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset + 8 <= size {
        out.push(Instr::AllocateStack(8));
        out.push(Instr::Movq {
            src: add_offset(src.clone(), offset),
            dst: Operand::Reg(Reg::R10),
        });
        out.push(Instr::Movq {
            src: Operand::Reg(Reg::R10),
            dst: Operand::Memory(Reg::SP, 0),
        });
        offset += 8;
    }
    if offset < size {
        out.push(Instr::AllocateStack(8));
        out.push(Instr::Movq {
            src: Operand::Imm(0),
            dst: Operand::Memory(Reg::SP, 0),
        });
    }
    while offset < size {
        out.push(Instr::MovByte {
            src: add_offset(src.clone(), offset),
            dst: Operand::Reg(Reg::R10),
        });
        out.push(Instr::MovByte {
            src: Operand::Reg(Reg::R10),
            dst: Operand::Memory(Reg::SP, (offset % 8) as i32),
        });
        offset += 1;
    }
    out
}

fn copy_bytes_to_address(src: Operand, dst: Operand, size: i64) -> Vec<Instr> {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset + 8 <= size {
        out.push(Instr::Movq {
            src: add_offset(src.clone(), offset),
            dst: Operand::Reg(Reg::R10),
        });
        out.push(Instr::Movq {
            src: Operand::Reg(Reg::R10),
            dst: add_offset(dst.clone(), offset),
        });
        offset += 8;
    }
    while offset < size {
        out.push(Instr::MovByte {
            src: add_offset(src.clone(), offset),
            dst: Operand::Reg(Reg::R10),
        });
        out.push(Instr::MovByte {
            src: Operand::Reg(Reg::R10),
            dst: add_offset(dst.clone(), offset),
        });
        offset += 1;
    }
    out
}

fn ast_type_of_val(val: &Val, ast_env: &HashMap<String, Type>) -> Type {
    match val {
        Val::Constant(_) => Type::Int,
        Val::ConstantDouble(_) => Type::Double,
        Val::Var(name) => ast_env.get(name).cloned().unwrap_or(Type::Int),
    }
}

fn lower_call(
    name: &str,
    args: &[Val],
    dst: &Option<String>,
    type_env: &TypeEnv,
    ast_env: &HashMap<String, Type>,
    ctx: &mut CodegenCtx,
) -> Vec<Instr> {
    let param_types = ctx
        .function_param_types
        .get(name)
        .cloned()
        .unwrap_or_else(|| args.iter().map(|v| ast_type_of_val(v, ast_env)).collect());
    let ret_type = dst
        .as_ref()
        .and_then(|_| ctx.function_return_types.get(name).cloned())
        .unwrap_or(Type::Void);
    let return_on_stack =
        matches!(ret_type, Type::Struct(_) | Type::Union(_)) && abi::returns_on_stack(&ret_type);
    let classified = abi::classify_typed_parameters(&param_types, return_on_stack);
    let mut out: Vec<Instr> = Vec::new();

    if return_on_stack && let Some(dst_name) = dst {
        out.push(Instr::Lea {
            src: Operand::Pseudo(dst_name.clone()),
            dst: Operand::Reg(Reg::DI),
        });
    }

    let stack_arg_count = classified.stack_slots.len();
    let padding = if stack_arg_count.is_multiple_of(2) {
        0
    } else {
        8
    };
    if padding != 0 {
        out.push(Instr::AllocateStack(padding));
    }

    let first_int_reg = if return_on_stack { 1 } else { 0 };
    for (idx, slot) in classified.int_slots.iter().enumerate() {
        let reg = abi_reg(abi::int_param_reg(idx + first_int_reg));
        let val = &args[slot.param_index];
        if matches!(
            param_types.get(slot.param_index),
            Some(Type::Struct(_) | Type::Union(_))
        ) {
            out.extend(copy_mem_to_reg(
                slot_operand(val, slot.offset, ctx),
                slot.size,
                reg,
            ));
        } else {
            let arg_ctx = copy_prop_support::IntArgMoveCtx {
                args,
                param_types: &param_types,
                classified: &classified,
                type_env,
                first_int_reg,
            };
            out.push(
                copy_prop_support::move_reused_int_arg(idx, val, reg.clone(), &arg_ctx)
                    .unwrap_or_else(|| {
                        copy_prop_support::move_call_arg(
                            val,
                            reg,
                            param_types.get(slot.param_index),
                            type_env,
                            ctx,
                        )
                    }),
            );
        }
    }

    for (idx, slot) in classified.sse_slots.iter().enumerate() {
        let val = &args[slot.param_index];
        let src = if matches!(
            param_types.get(slot.param_index),
            Some(Type::Struct(_) | Type::Union(_))
        ) {
            slot_operand(val, slot.offset, ctx)
        } else {
            convert_val(val, ctx)
        };
        out.push(Instr::Movsd {
            src,
            dst: Operand::Reg(Reg::XMM(idx as u8)),
        });
    }

    for slot in classified.stack_slots.iter().rev() {
        let val = &args[slot.param_index];
        if matches!(
            param_types.get(slot.param_index),
            Some(Type::Struct(_) | Type::Union(_))
        ) {
            out.extend(copy_mem_to_stack(
                slot_operand(val, slot.offset, ctx),
                slot.size,
            ));
        } else {
            let ty = type_of_val(val, type_env);
            if ty == OperandType::Byte {
                out.push(Instr::MovSignExtendByte {
                    src: convert_val(val, ctx),
                    dst: Operand::Reg(Reg::R10),
                });
                out.push(Instr::Push(Operand::Reg(Reg::R10)));
            } else if ty == OperandType::UByte {
                out.push(Instr::MovZeroExtend {
                    src: convert_val(val, ctx),
                    dst: Operand::Reg(Reg::R10),
                });
                out.push(Instr::Push(Operand::Reg(Reg::R10)));
            } else if ty == OperandType::Double {
                out.push(Instr::AllocateStack(8));
                out.push(Instr::Movsd {
                    src: convert_val(val, ctx),
                    dst: Operand::Memory(Reg::SP, 0),
                });
            } else {
                out.push(Instr::Push(convert_val(val, ctx)));
            }
        }
    }

    out.push(Instr::Call(name.to_string()));

    let bytes_to_remove = padding + 8 * (stack_arg_count as i32);
    if bytes_to_remove != 0 {
        out.push(Instr::DeallocateStack(bytes_to_remove));
    }

    if let Some(dst_name) = dst {
        if return_on_stack {
            return out;
        }
        match &ret_type {
            Type::Struct(_) | Type::Union(_) => {
                let classes = abi::classify_aggregate(&ret_type);
                let mut int_idx = 0usize;
                let mut sse_idx = 0usize;
                for (eight_idx, class) in classes.iter().enumerate() {
                    let dst_op = Operand::PseudoMem(dst_name.clone(), (eight_idx as i32) * 8);
                    let size = abi::eightbyte_size(ret_type.clone().size(), eight_idx);
                    match class {
                        abi::EightbyteClass::Integer => {
                            let reg = if int_idx == 0 { Reg::AX } else { Reg::DX };
                            out.extend(copy_reg_to_mem(reg, dst_op, size));
                            int_idx += 1;
                        }
                        abi::EightbyteClass::Sse => {
                            out.push(Instr::Movsd {
                                src: Operand::Reg(Reg::XMM(sse_idx as u8)),
                                dst: dst_op,
                            });
                            sse_idx += 1;
                        }
                        abi::EightbyteClass::Memory => {}
                    }
                }
            }
            Type::Double => out.push(Instr::Movsd {
                src: Operand::Reg(Reg::XMM(0)),
                dst: Operand::Pseudo(dst_name.clone()),
            }),
            _ => {
                let dst_ty = type_of_val(&Val::Var(dst_name.clone()), type_env);
                if is_byte_type(dst_ty) {
                    out.push(Instr::MovByte {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst_name.clone()),
                    });
                } else if dst_ty.is_long_word() || matches!(ret_type, Type::Pointer(_)) {
                    out.push(Instr::Movq {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst_name.clone()),
                    });
                } else {
                    out.push(Instr::Mov {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst_name.clone()),
                    });
                }
            }
        }
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
fn lower_instruction(
    instr: &Instruction,
    env: &TypeEnv,
    ast_env: &HashMap<String, Type>,
    ctx: &mut CodegenCtx,
) -> Vec<Instr> {
    match instr {
        Instruction::Return(val) => {
            let ast_ty = ast_type_of_val(val, ast_env);
            if matches!(ast_ty, Type::Struct(_) | Type::Union(_)) {
                let mut out = Vec::new();
                if ctx.current_return_on_stack {
                    out.push(Instr::Movq {
                        src: Operand::Pseudo(format!("{}.return_ptr", ctx.current_return_name())),
                        dst: Operand::Reg(Reg::R9),
                    });
                    out.extend(copy_bytes_to_address(
                        slot_operand(val, 0, ctx),
                        Operand::Memory(Reg::R9, 0),
                        ast_ty.size(),
                    ));
                    out.push(Instr::Movq {
                        src: Operand::Reg(Reg::R9),
                        dst: Operand::Reg(Reg::AX),
                    });
                } else {
                    let classes = abi::classify_aggregate(&ast_ty);
                    let mut int_idx = 0usize;
                    let mut sse_idx = 0usize;
                    for (eight_idx, class) in classes.iter().enumerate() {
                        let src = slot_operand(val, (eight_idx as i64) * 8, ctx);
                        let size = abi::eightbyte_size(ast_ty.clone().size(), eight_idx);
                        match class {
                            abi::EightbyteClass::Integer => {
                                let reg = if int_idx == 0 { Reg::AX } else { Reg::DX };
                                out.extend(copy_mem_to_reg(src, size, reg));
                                int_idx += 1;
                            }
                            abi::EightbyteClass::Sse => {
                                out.push(Instr::Movsd {
                                    src,
                                    dst: Operand::Reg(Reg::XMM(sse_idx as u8)),
                                });
                                sse_idx += 1;
                            }
                            abi::EightbyteClass::Memory => {}
                        }
                    }
                }
                out.push(Instr::Ret);
                return out;
            }
            let val_ty = match val {
                Val::Constant(_) => ctx
                    .current_return_type
                    .as_ref()
                    .map_or(OperandType::Int, operand_type_for_scalar_return),
                Val::ConstantDouble(_) | Val::Var(_) => type_of_val(val, env),
            };
            let is_long = val_ty.is_long_word();
            let mut out: Vec<Instr> = Vec::new();
            if val_ty == OperandType::Double {
                out.push(Instr::Movsd {
                    src: convert_val(val, ctx),
                    dst: Operand::Reg(Reg::XMM(0)),
                });
            } else if is_byte_type(val_ty) {
                out.push(Instr::MovByte {
                    src: convert_val(val, ctx),
                    dst: Operand::Reg(Reg::AX),
                });
            } else if is_long {
                if let Val::Constant(n) = val
                    && immediate_too_wide(*n)
                {
                    out.push(Instr::Movabsq {
                        src: *n,
                        dst: Operand::Reg(Reg::AX),
                    });
                    out.push(Instr::Ret);
                    return out;
                }
                out.push(Instr::Movq {
                    src: convert_val(val, ctx),
                    dst: Operand::Reg(Reg::AX),
                });
            } else {
                out.push(Instr::Mov {
                    src: convert_val(val, ctx),
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
            if dst_ty == OperandType::Double {
                vec![Instr::Movsd {
                    src: convert_val(src, ctx),
                    dst: Operand::Pseudo(dst.clone()),
                }]
            } else if is_byte_type(dst_ty) {
                vec![Instr::MovByte {
                    src: convert_val(src, ctx),
                    dst: Operand::Pseudo(dst.clone()),
                }]
            } else if dst_ty.is_long_word() {
                // 64-bit immediates outside the i32 range need
                // `movabsq` into a register first; `movq imm32, mem`
                // is the only direct immediate form.
                if let Val::Constant(n) = src
                    && immediate_too_wide(*n)
                {
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
                vec![Instr::Movq {
                    src: convert_val(src, ctx),
                    dst: Operand::Pseudo(dst.clone()),
                }]
            } else {
                vec![Instr::Mov {
                    src: convert_val(src, ctx),
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
            let src_op = convert_val(src, ctx);
            if is_byte_type(type_of_val(src, env)) {
                let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
                if dst_ty.is_long_word() {
                    return vec![
                        Instr::MovSignExtendByte {
                            src: src_op,
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
                    ];
                }
                return vec![Instr::MovSignExtendByte {
                    src: src_op,
                    dst: Operand::Pseudo(dst.clone()),
                }];
            }
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
        Instruction::ZeroExtend { src, dst } => {
            // Mirrors OCaml `backend/codegen.ml` `Tacky.ZeroExtend`
            // plus `instruction_fixup.ml`: zero-extending an
            // unsigned int to a long/ulong is just a 32-bit move into
            // a register (x86-64 clears the high half), followed by a
            // 64-bit move into the destination pseudo.  Emitting the
            // explicit pair here keeps the existing assembly IR's
            // byte-oriented `MovZeroExtend` for `setcc` results.
            if is_byte_type(type_of_val(src, env)) {
                let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
                if dst_ty.is_long_word() {
                    return vec![
                        Instr::MovZeroExtend {
                            src: convert_val(src, ctx),
                            dst: Operand::Reg(Reg::R10),
                        },
                        Instr::Movq {
                            src: Operand::Reg(Reg::R10),
                            dst: Operand::Pseudo(dst.clone()),
                        },
                    ];
                }
                return vec![Instr::MovZeroExtend {
                    src: convert_val(src, ctx),
                    dst: Operand::Pseudo(dst.clone()),
                }];
            }
            vec![
                Instr::Mov {
                    src: convert_val(src, ctx),
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Movq {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Pseudo(dst.clone()),
                },
            ]
        }
        // Truncate a `long` value to `int` by routing through a
        // 32-bit move.  `movq src, %r10d` reads the low 32 bits
        // into %r10d (zero-extends in x86-64); `movl %r10d, dst`
        // writes the truncated value.  For pointers (still 64-bit)
        // this lowers to a plain `movq`; for plain `long` it
        // narrows to 32 bits.
        Instruction::Truncate { src, dst } => {
            vec![if is_byte_type(type_of_val(&Val::Var(dst.clone()), env)) {
                Instr::MovByte {
                    src: convert_val(src, ctx),
                    dst: Operand::Pseudo(dst.clone()),
                }
            } else {
                Instr::Mov {
                    src: convert_val(src, ctx),
                    dst: Operand::Pseudo(dst.clone()),
                }
            }]
        }
        Instruction::IntToDouble { src, dst } => {
            let src_ty = type_of_val(src, env);
            let src_op = convert_val(src, ctx);
            let cvt_src = if src_ty == OperandType::Long {
                src_op
            } else if src_ty == OperandType::Byte {
                return vec![
                    Instr::MovSignExtendByte {
                        src: src_op,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Movsx {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            } else if matches!(src_op, Operand::Imm(_)) {
                return vec![
                    Instr::Mov {
                        src: src_op,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            } else {
                return vec![
                    Instr::Movsx {
                        src: src_op,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            };
            vec![Instr::Cvtsi2sd {
                src: cvt_src,
                dst: Operand::Pseudo(dst.clone()),
            }]
        }
        Instruction::UIntToDouble { src, dst } => {
            let src_ty = type_of_val(src, env);
            let src_op = convert_val(src, ctx);
            if src_ty == OperandType::UByte {
                vec![
                    Instr::MovZeroExtend {
                        src: src_op,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ]
            } else if src_ty == OperandType::UInt {
                vec![
                    Instr::Mov {
                        src: src_op,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ]
            } else if src_ty == OperandType::ULong {
                let out_of_range = ctx.fresh_label("ulong_to_double.out_of_range");
                let end = ctx.fresh_label("ulong_to_double.end");
                vec![
                    Instr::Movq {
                        src: src_op,
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::Cmpq {
                        left: Operand::Reg(Reg::R10),
                        right: Operand::Imm(0),
                    },
                    Instr::JmpCC {
                        cc: AsmCC::L,
                        label: out_of_range.clone(),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                    Instr::Jmp(end.clone()),
                    Instr::Label(out_of_range),
                    Instr::Movq {
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Reg(Reg::R11),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::BitAndQ,
                        src: Operand::Imm(1),
                        dst: Operand::Reg(Reg::R10),
                    },
                    Instr::UnaryQ {
                        op: UnaryOpInstr::Shr,
                        operand: Operand::Reg(Reg::R11),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::BitOrQ,
                        src: Operand::Reg(Reg::R10),
                        dst: Operand::Reg(Reg::R11),
                    },
                    Instr::Cvtsi2sd {
                        src: Operand::Reg(Reg::R11),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                    Instr::Movsd {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::AddDouble,
                        src: Operand::Reg(Reg::XMM(14)),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::Movsd {
                        src: Operand::Reg(Reg::XMM(14)),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                    Instr::Label(end),
                ]
            } else {
                vec![Instr::Cvtsi2sd {
                    src: src_op,
                    dst: Operand::Pseudo(dst.clone()),
                }]
            }
        }
        Instruction::DoubleToInt { src, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            let mut out = vec![Instr::Cvttsd2si {
                src: convert_val(src, ctx),
                dst: Operand::Reg(Reg::R10),
            }];
            if dst_ty.is_long_word() {
                out.push(Instr::Movq {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Pseudo(dst.clone()),
                });
            } else if is_byte_type(dst_ty) {
                out.push(Instr::MovByte {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Pseudo(dst.clone()),
                });
            } else {
                out.push(Instr::Mov {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Pseudo(dst.clone()),
                });
            }
            out
        }
        Instruction::DoubleToUInt { src, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty != OperandType::ULong {
                return vec![
                    Instr::Cvttsd2si {
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::R10),
                    },
                    if is_byte_type(dst_ty) {
                        Instr::MovByte {
                            src: Operand::Reg(Reg::R10),
                            dst: Operand::Pseudo(dst.clone()),
                        }
                    } else {
                        Instr::Mov {
                            src: Operand::Reg(Reg::R10),
                            dst: Operand::Pseudo(dst.clone()),
                        }
                    },
                ];
            }
            let above = ctx.fresh_label("double_to_ulong.above");
            let end = ctx.fresh_label("double_to_ulong.end");
            let threshold = Operand::Data(ctx.double_label(9223372036854775808.0));
            vec![
                Instr::Movsd {
                    src: convert_val(src, ctx),
                    dst: Operand::Reg(Reg::XMM(14)),
                },
                Instr::CmpDouble {
                    left: Operand::Reg(Reg::XMM(14)),
                    right: threshold.clone(),
                },
                Instr::JmpCC {
                    cc: AsmCC::AE,
                    label: above.clone(),
                },
                Instr::Cvttsd2si {
                    src: Operand::Reg(Reg::XMM(14)),
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Jmp(end.clone()),
                Instr::Label(above),
                Instr::BinaryOp {
                    op: BinaryOpInstr::SubDouble,
                    src: threshold,
                    dst: Operand::Reg(Reg::XMM(14)),
                },
                Instr::Cvttsd2si {
                    src: Operand::Reg(Reg::XMM(14)),
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Movabsq {
                    src: i64::MIN,
                    dst: Operand::Reg(Reg::R11),
                },
                Instr::BinaryOp {
                    op: BinaryOpInstr::AddQ,
                    src: Operand::Reg(Reg::R11),
                    dst: Operand::Reg(Reg::R10),
                },
                Instr::Label(end),
                Instr::Movq {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Pseudo(dst.clone()),
                },
            ]
        }
        Instruction::Negate { dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty == OperandType::Double {
                let sign_mask = Operand::Data(ctx.double_label(-0.0));
                return vec![
                    Instr::Movsd {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::XMM(1)),
                    },
                    Instr::Movsd {
                        src: sign_mask,
                        dst: Operand::Reg(Reg::XMM(15)),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::XorDouble,
                        src: Operand::Reg(Reg::XMM(15)),
                        dst: Operand::Reg(Reg::XMM(1)),
                    },
                    Instr::Movsd {
                        src: Operand::Reg(Reg::XMM(1)),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            }
            let is_long = dst_ty.is_long_word();
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
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty == OperandType::Double {
                return vec![
                    Instr::Movsd {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::AddDouble,
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::Movsd {
                        src: Operand::Reg(Reg::XMM(14)),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            }
            let op = if dst_ty == OperandType::Double {
                BinaryOpInstr::AddDouble
            } else if dst_ty.is_long_word() {
                BinaryOpInstr::AddQ
            } else {
                BinaryOpInstr::Add
            };
            vec![Instr::BinaryOp {
                op,
                src: convert_val(src, ctx),
                dst: Operand::Pseudo(dst.clone()),
            }]
        }
        Instruction::Sub { src, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty == OperandType::Double {
                return vec![
                    Instr::Movsd {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::SubDouble,
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::Movsd {
                        src: Operand::Reg(Reg::XMM(14)),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            }
            let op = if dst_ty == OperandType::Double {
                BinaryOpInstr::SubDouble
            } else if dst_ty.is_long_word() {
                BinaryOpInstr::SubQ
            } else {
                BinaryOpInstr::Sub
            };
            vec![Instr::BinaryOp {
                op,
                src: convert_val(src, ctx),
                dst: Operand::Pseudo(dst.clone()),
            }]
        }
        Instruction::Mul { src, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty == OperandType::Double {
                return vec![
                    Instr::Movsd {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::MultDouble,
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::Movsd {
                        src: Operand::Reg(Reg::XMM(14)),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            }
            let op = if dst_ty.is_long_word() {
                BinaryOpInstr::MultQ
            } else {
                BinaryOpInstr::Mult
            };
            vec![Instr::BinaryOp {
                op,
                src: convert_val(src, ctx),
                dst: Operand::Pseudo(dst.clone()),
            }]
        }
        Instruction::DivSigned { src, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            if dst_ty == OperandType::Double {
                return vec![
                    Instr::Movsd {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::BinaryOp {
                        op: BinaryOpInstr::SseDivDouble,
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::XMM(14)),
                    },
                    Instr::Movsd {
                        src: Operand::Reg(Reg::XMM(14)),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ];
            }
            let is_long = dst_ty.is_long_word();
            let is_unsigned = dst_ty.is_unsigned();
            if is_long {
                vec![
                    Instr::Movq {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::AX),
                    },
                    Instr::Movq {
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::R10),
                    },
                    if is_unsigned {
                        Instr::Movq {
                            src: Operand::Imm(0),
                            dst: Operand::Reg(Reg::DX),
                        }
                    } else {
                        Instr::Cqo
                    },
                    if is_unsigned {
                        Instr::Divq(Operand::Reg(Reg::R10))
                    } else {
                        Instr::Idivq(Operand::Reg(Reg::R10))
                    },
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
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::R10),
                    },
                    if is_unsigned {
                        Instr::Mov {
                            src: Operand::Imm(0),
                            dst: Operand::Reg(Reg::DX),
                        }
                    } else {
                        Instr::Cdq
                    },
                    if is_unsigned {
                        Instr::Div(Operand::Reg(Reg::R10))
                    } else {
                        Instr::Idiv(Operand::Reg(Reg::R10))
                    },
                    Instr::Mov {
                        src: Operand::Reg(Reg::AX),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ]
            }
        }
        Instruction::RemSigned { src, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            let is_long = dst_ty.is_long_word();
            let is_unsigned = dst_ty.is_unsigned();
            if is_long {
                vec![
                    Instr::Movq {
                        src: Operand::Pseudo(dst.clone()),
                        dst: Operand::Reg(Reg::AX),
                    },
                    Instr::Movq {
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::R10),
                    },
                    if is_unsigned {
                        Instr::Movq {
                            src: Operand::Imm(0),
                            dst: Operand::Reg(Reg::DX),
                        }
                    } else {
                        Instr::Cqo
                    },
                    if is_unsigned {
                        Instr::Divq(Operand::Reg(Reg::R10))
                    } else {
                        Instr::Idivq(Operand::Reg(Reg::R10))
                    },
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
                        src: convert_val(src, ctx),
                        dst: Operand::Reg(Reg::R10),
                    },
                    if is_unsigned {
                        Instr::Mov {
                            src: Operand::Imm(0),
                            dst: Operand::Reg(Reg::DX),
                        }
                    } else {
                        Instr::Cdq
                    },
                    if is_unsigned {
                        Instr::Div(Operand::Reg(Reg::R10))
                    } else {
                        Instr::Idiv(Operand::Reg(Reg::R10))
                    },
                    Instr::Mov {
                        src: Operand::Reg(Reg::DX),
                        dst: Operand::Pseudo(dst.clone()),
                    },
                ]
            }
        }
        Instruction::BitAnd { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::BitAnd,
            src: convert_val(src, ctx),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::BitOr { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::BitOr,
            src: convert_val(src, ctx),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::BitXor { src, dst } => vec![Instr::BinaryOp {
            op: BinaryOpInstr::BitXor,
            src: convert_val(src, ctx),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::BitShiftLeft { src, dst } => vec![
            Instr::Mov {
                src: convert_val(src, ctx),
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
                src: convert_val(src, ctx),
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
        Instruction::Cmp {
            left,
            right,
            dst,
            cc,
        } => {
            let left_op = convert_val(left, ctx);
            let right_op = convert_val(right, ctx);
            let left_ty = type_of_val(left, env);
            let right_ty = type_of_val(right, env);
            let is_double = left_ty == OperandType::Double || right_ty == OperandType::Double;
            let is_unsigned = left_ty.is_unsigned() || right_ty.is_unsigned();
            let is_long = left_ty.is_long_word() || right_ty.is_long_word();
            let right_op = if is_double && matches!(right_op, Operand::Imm(0)) {
                Operand::Data(ctx.double_label(0.0))
            } else {
                right_op
            };
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
            if is_double {
                out.push(Instr::Movsd {
                    src: cmp_left,
                    dst: Operand::Reg(Reg::XMM(14)),
                });
                out.push(Instr::CmpDouble {
                    left: Operand::Reg(Reg::XMM(14)),
                    right: right_op,
                });
                out.extend(set_double_comparison(*cc, dst, ctx));
            } else if is_long {
                let right_op = match right_op {
                    Operand::Imm(n) if immediate_too_wide(n) => {
                        out.push(Instr::Movabsq {
                            src: n,
                            dst: Operand::Reg(Reg::R10),
                        });
                        Operand::Reg(Reg::R10)
                    }
                    other => other,
                };
                out.push(Instr::Cmpq {
                    left: cmp_left,
                    right: right_op,
                });
                out.push(Instr::SetCC {
                    cc: map_integer_cc(*cc, is_unsigned),
                    dst: Operand::Pseudo(dst.clone()),
                });
            } else {
                out.push(Instr::Cmp {
                    left: cmp_left,
                    right: right_op,
                });
                out.push(Instr::SetCC {
                    cc: map_integer_cc(*cc, is_unsigned),
                    dst: Operand::Pseudo(dst.clone()),
                });
            }
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
            let cond_op = convert_val(condition, ctx);
            let cond_ty = type_of_val(condition, env);
            let is_double = cond_ty == OperandType::Double;
            let is_long = cond_ty.is_long_word();
            let (prelude, cmp_left) = if is_byte_type(cond_ty) {
                (
                    vec![Instr::MovSignExtendByte {
                        src: cond_op,
                        dst: Operand::Reg(Reg::R10),
                    }],
                    Operand::Reg(Reg::R10),
                )
            } else {
                match &cond_op {
                    Operand::Imm(_) if !is_double => {
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
                }
            };
            let mut out = prelude;
            if is_double {
                let zero = Operand::Data(ctx.double_label(0.0));
                let unordered = ctx.fresh_label("double_jump_if_zero.unordered");
                out.push(Instr::Movsd {
                    src: cmp_left,
                    dst: Operand::Reg(Reg::XMM(14)),
                });
                out.push(Instr::CmpDouble {
                    left: Operand::Reg(Reg::XMM(14)),
                    right: zero,
                });
                out.push(Instr::JmpCC {
                    cc: AsmCC::P,
                    label: unordered.clone(),
                });
                out.push(Instr::JmpCC {
                    cc: AsmCC::E,
                    label: target.clone(),
                });
                out.push(Instr::Label(unordered));
                return out;
            } else if is_long {
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
        }
        // Chapter 4 short-circuit `||` lowering materialises
        // `JumpIfNotZero`.  Codegen turns this into
        //   cmpl $0, cond
        //   jne  target
        // Same immediate-handling workaround as `JumpIfZero` above.
        // Chapter 11: when `condition` is `long`, use `cmpq`.
        Instruction::JumpIfNotZero { condition, target } => {
            let cond_op = convert_val(condition, ctx);
            let cond_ty = type_of_val(condition, env);
            let is_double = cond_ty == OperandType::Double;
            let is_long = cond_ty.is_long_word();
            let (prelude, cmp_left) = if is_byte_type(cond_ty) {
                (
                    vec![Instr::MovSignExtendByte {
                        src: cond_op,
                        dst: Operand::Reg(Reg::R10),
                    }],
                    Operand::Reg(Reg::R10),
                )
            } else {
                match &cond_op {
                    Operand::Imm(_) if !is_double => {
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
                }
            };
            let mut out = prelude;
            if is_double {
                let zero = Operand::Data(ctx.double_label(0.0));
                out.push(Instr::Movsd {
                    src: cmp_left,
                    dst: Operand::Reg(Reg::XMM(14)),
                });
                out.push(Instr::CmpDouble {
                    left: Operand::Reg(Reg::XMM(14)),
                    right: zero,
                });
                out.push(Instr::JmpCC {
                    cc: AsmCC::P,
                    label: target.clone(),
                });
                out.push(Instr::JmpCC {
                    cc: AsmCC::NE,
                    label: target.clone(),
                });
                return out;
            } else if is_long {
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
        }
        // Chapter 4 short-circuit `&&` / `||` lowering materialises
        // forward labels for the join point.
        Instruction::Label(name) => vec![Instr::Label(name.clone())],
        Instruction::Load { src_pointer, dst } => {
            let dst_ty = type_of_val(&Val::Var(dst.clone()), env);
            let dst_op = Operand::Pseudo(dst.clone());
            let load = if dst_ty == OperandType::Double {
                Instr::Movsd {
                    src: Operand::Memory(Reg::R9, 0),
                    dst: dst_op,
                }
            } else if is_byte_type(dst_ty) {
                Instr::MovByte {
                    src: Operand::Memory(Reg::R9, 0),
                    dst: dst_op,
                }
            } else if dst_ty.is_long_word() {
                Instr::Movq {
                    src: Operand::Memory(Reg::R9, 0),
                    dst: dst_op,
                }
            } else {
                Instr::Mov {
                    src: Operand::Memory(Reg::R9, 0),
                    dst: dst_op,
                }
            };
            vec![
                Instr::Movq {
                    src: convert_val(src_pointer, ctx),
                    dst: Operand::Reg(Reg::R9),
                },
                load,
            ]
        }
        Instruction::Store { src, dst_pointer } => {
            let src_ty = type_of_val(src, env);
            let src_op = convert_val(src, ctx);
            let store = if src_ty == OperandType::Double {
                Instr::Movsd {
                    src: src_op,
                    dst: Operand::Memory(Reg::R9, 0),
                }
            } else if is_byte_type(src_ty) {
                Instr::MovByte {
                    src: src_op,
                    dst: Operand::Memory(Reg::R9, 0),
                }
            } else if src_ty.is_long_word() {
                Instr::Movq {
                    src: src_op,
                    dst: Operand::Memory(Reg::R9, 0),
                }
            } else {
                Instr::Mov {
                    src: src_op,
                    dst: Operand::Memory(Reg::R9, 0),
                }
            };
            vec![
                Instr::Movq {
                    src: convert_val(dst_pointer, ctx),
                    dst: Operand::Reg(Reg::R9),
                },
                store,
            ]
        }
        Instruction::CopyBytes {
            src_pointer,
            dst_pointer,
            size,
        } => {
            let mut out = vec![
                Instr::Movq {
                    src: convert_val(src_pointer, ctx),
                    dst: Operand::Reg(Reg::R8),
                },
                Instr::Movq {
                    src: convert_val(dst_pointer, ctx),
                    dst: Operand::Reg(Reg::R9),
                },
            ];
            let mut offset = 0;
            while offset + 8 <= *size {
                out.push(Instr::Movq {
                    src: Operand::Memory(Reg::R8, offset as i32),
                    dst: Operand::Reg(Reg::R10),
                });
                out.push(Instr::Movq {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Memory(Reg::R9, offset as i32),
                });
                offset += 8;
            }
            while offset < *size {
                out.push(Instr::MovByte {
                    src: Operand::Memory(Reg::R8, offset as i32),
                    dst: Operand::Reg(Reg::R10),
                });
                out.push(Instr::MovByte {
                    src: Operand::Reg(Reg::R10),
                    dst: Operand::Memory(Reg::R9, offset as i32),
                });
                offset += 1;
            }
            out
        }
        Instruction::GetAddress { src, dst } => vec![Instr::Lea {
            src: Operand::Pseudo(src.clone()),
            dst: Operand::Pseudo(dst.clone()),
        }],
        Instruction::AddPtr {
            ptr,
            index,
            scale,
            dst,
        } => {
            let index_op = convert_val(index, ctx);
            let mut out = Vec::new();
            if let Some(lowered) =
                copy_prop_support::lower_const_index_addptr(ptr, &index_op, *scale, dst, ctx)
            {
                return lowered;
            }
            match index_op {
                op => {
                    let index_ty = type_of_val(index, env);
                    if index_ty.is_long_word() {
                        out.push(Instr::Movq {
                            src: op,
                            dst: Operand::Reg(Reg::R11),
                        });
                    } else if is_byte_type(index_ty) {
                        out.push(Instr::MovSignExtendByte {
                            src: op,
                            dst: Operand::Reg(Reg::R11),
                        });
                    } else {
                        out.push(Instr::Movsx {
                            src: op,
                            dst: Operand::Reg(Reg::R11),
                        });
                    }
                }
            }
            let sib_scale = if matches!(*scale, 1 | 2 | 4 | 8) {
                *scale as i32
            } else {
                out.push(Instr::BinaryOp {
                    op: BinaryOpInstr::MultQ,
                    src: Operand::Imm(*scale),
                    dst: Operand::Reg(Reg::R11),
                });
                1
            };
            out.push(Instr::Movq {
                src: convert_val(ptr, ctx),
                dst: Operand::Reg(Reg::R10),
            });
            out.push(Instr::Lea {
                src: Operand::MemoryIndexed(Reg::R10, Reg::R11, sib_scale),
                dst: Operand::Pseudo(dst.clone()),
            });
            out
        }
        Instruction::Call { name, args, dst } => lower_call(name, args, dst, env, ast_env, ctx),
        _ => Vec::new(),
    }
}

fn collect_global_copy_dests(body: &[Instruction], globals: &TypeEnv) -> BTreeSet<String> {
    let mut pointer_uses = HashMap::<String, usize>::new();
    let mut copy_dests = BTreeSet::new();
    for instruction in body {
        match instruction {
            Instruction::Store { dst_pointer, .. } => {
                count_pointer_use(dst_pointer, &mut pointer_uses)
            }
            Instruction::Load { src_pointer, .. } => {
                count_pointer_use(src_pointer, &mut pointer_uses)
            }
            Instruction::CopyBytes {
                src_pointer,
                dst_pointer,
                ..
            } => {
                count_pointer_use(src_pointer, &mut pointer_uses);
                if let Val::Var(name) = dst_pointer {
                    copy_dests.insert(name.clone());
                }
                count_pointer_use(dst_pointer, &mut pointer_uses);
            }
            Instruction::AddPtr { ptr, .. } => count_pointer_use(ptr, &mut pointer_uses),
            _ => {}
        }
    }
    body.iter()
        .filter_map(|instruction| match instruction {
            Instruction::GetAddress { src, dst }
                if globals.contains_key(src)
                    && copy_dests.contains(dst)
                    && pointer_uses.get(dst).copied() == Some(1) =>
            {
                Some(dst.clone())
            }
            _ => None,
        })
        .collect()
}

fn count_pointer_use(val: &Val, pointer_uses: &mut HashMap<String, usize>) {
    if let Val::Var(name) = val {
        *pointer_uses.entry(name.clone()).or_insert(0) += 1;
    }
}

fn lower_copybytes_to_global(
    src_pointer: &Val,
    dst: &str,
    size: i64,
    ctx: &mut CodegenCtx,
) -> Vec<Instr> {
    let mut out = vec![Instr::Movq {
        src: convert_val(src_pointer, ctx),
        dst: Operand::Reg(Reg::R8),
    }];
    let mut offset = 0;
    while offset + 8 <= size {
        out.push(Instr::Movq {
            src: Operand::Memory(Reg::R8, offset as i32),
            dst: Operand::Reg(Reg::R10),
        });
        out.push(Instr::Movq {
            src: Operand::Reg(Reg::R10),
            dst: Operand::DataOffset(dst.to_string(), offset as i32),
        });
        offset += 8;
    }
    while offset < size {
        out.push(Instr::MovByte {
            src: Operand::Memory(Reg::R8, offset as i32),
            dst: Operand::Reg(Reg::R10),
        });
        out.push(Instr::MovByte {
            src: Operand::Reg(Reg::R10),
            dst: Operand::DataOffset(dst.to_string(), offset as i32),
        });
        offset += 1;
    }
    out
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
fn generate_function(func: &TackyFunction, globals: &TypeEnv, ctx: &mut CodegenCtx) -> TopLevel {
    ctx.current_function_name = func.name.clone();
    ctx.current_return_type = Some(func.return_type.clone());
    ctx.current_return_on_stack = matches!(func.return_type, Type::Struct(_) | Type::Union(_))
        && abi::returns_on_stack(&func.return_type);

    let param_tys: Vec<Type> = func
        .params
        .iter()
        .map(|p| func.ast_type_env.get(p).cloned().unwrap_or(Type::Int))
        .collect();
    let classified = abi::classify_typed_parameters(&param_tys, ctx.current_return_on_stack);
    let mut prologue: Vec<Instr> = Vec::new();
    if ctx.current_return_on_stack {
        prologue.push(Instr::Movq {
            src: Operand::Reg(Reg::DI),
            dst: Operand::Pseudo(format!("{}.return_ptr", func.name)),
        });
    }

    let first_int_reg = if ctx.current_return_on_stack { 1 } else { 0 };
    for (idx, slot) in classified.int_slots.iter().enumerate() {
        let param_name = &func.params[slot.param_index];
        let dst = Operand::PseudoMem(param_name.clone(), slot.offset as i32);
        if matches!(
            param_tys.get(slot.param_index),
            Some(Type::Struct(_) | Type::Union(_))
        ) {
            let reg = abi_reg(abi::int_param_reg(idx + first_int_reg));
            prologue.extend(copy_reg_to_mem(reg, dst, slot.size));
        } else {
            let param_ty = func
                .type_env
                .get(param_name)
                .copied()
                .unwrap_or(OperandType::Int);
            let src = Operand::Reg(abi_reg(abi::int_param_reg(idx + first_int_reg)));
            if param_ty == OperandType::Double {
                prologue.push(Instr::Movsd {
                    src,
                    dst: Operand::Pseudo(param_name.clone()),
                });
            } else if is_byte_type(param_ty) {
                prologue.push(Instr::MovByte {
                    src,
                    dst: Operand::Pseudo(param_name.clone()),
                });
            } else if param_ty.is_long_word()
                || matches!(param_tys.get(slot.param_index), Some(Type::Pointer(_)))
            {
                prologue.push(Instr::Movq {
                    src,
                    dst: Operand::Pseudo(param_name.clone()),
                });
            } else {
                prologue.push(Instr::Mov {
                    src,
                    dst: Operand::Pseudo(param_name.clone()),
                });
            }
        }
    }

    for (idx, slot) in classified.sse_slots.iter().enumerate() {
        let param_name = &func.params[slot.param_index];
        let dst = if matches!(
            param_tys.get(slot.param_index),
            Some(Type::Struct(_) | Type::Union(_))
        ) {
            Operand::PseudoMem(param_name.clone(), slot.offset as i32)
        } else {
            Operand::Pseudo(param_name.clone())
        };
        prologue.push(Instr::Movsd {
            src: Operand::Reg(Reg::XMM(idx as u8)),
            dst,
        });
    }

    for (idx, slot) in classified.stack_slots.iter().enumerate() {
        let param_name = &func.params[slot.param_index];
        let src = Operand::Memory(Reg::BP, 16 + (idx as i32) * 8);
        if matches!(
            param_tys.get(slot.param_index),
            Some(Type::Struct(_) | Type::Union(_))
        ) {
            prologue.extend(copy_bytes_to_address(
                src,
                Operand::PseudoMem(param_name.clone(), slot.offset as i32),
                slot.size,
            ));
        } else {
            let param_ty = func
                .type_env
                .get(param_name)
                .copied()
                .unwrap_or(OperandType::Int);
            let dst = Operand::Pseudo(param_name.clone());
            if param_ty == OperandType::Double {
                prologue.push(Instr::Movsd { src, dst });
            } else if is_byte_type(param_ty) {
                prologue.push(Instr::MovByte { src, dst });
            } else if param_ty.is_long_word()
                || matches!(param_tys.get(slot.param_index), Some(Type::Pointer(_)))
            {
                prologue.push(Instr::Movq { src, dst });
            } else {
                prologue.push(Instr::Mov { src, dst });
            }
        }
    }

    let mut merged = globals.clone();
    for (k, v) in &func.type_env {
        merged.insert(k.clone(), *v);
    }
    let global_copy_dests = collect_global_copy_dests(&func.body, globals);
    let mut address_of = BTreeMap::<String, String>::new();
    let mut body = Vec::new();
    for instr in &func.body {
        match instr {
            Instruction::GetAddress { src, dst } if global_copy_dests.contains(dst) => {
                address_of.insert(dst.clone(), src.clone());
            }
            Instruction::GetAddress { src, dst } => {
                address_of.insert(dst.clone(), src.clone());
                body.extend(lower_instruction(instr, &merged, &func.ast_type_env, ctx));
            }
            Instruction::CopyBytes {
                src_pointer,
                dst_pointer: Val::Var(dst_pointer),
                size,
            } if address_of
                .get(dst_pointer)
                .is_some_and(|name| globals.contains_key(name)) =>
            {
                if let Some(dst) = address_of.get(dst_pointer) {
                    body.extend(lower_copybytes_to_global(src_pointer, dst, *size, ctx));
                }
            }
            _ => body.extend(lower_instruction(instr, &merged, &func.ast_type_env, ctx)),
        }
    }
    let mut type_env = func.type_env.clone();
    if ctx.current_return_on_stack {
        type_env.insert(format!("{}.return_ptr", func.name), OperandType::Long);
    }
    let mut instructions = Vec::with_capacity(prologue.len() + body.len());
    instructions.extend(prologue);
    instructions.extend(body);
    TopLevel::Fn {
        name: func.name.clone(),
        global: func.global,
        instructions,
        type_env,
    }
}

fn convert_static_init(
    init: crate::ir::tacky::TackyStaticInit,
    ty: OperandType,
) -> Vec<StaticInit> {
    match (init, ty) {
        (crate::ir::tacky::TackyStaticInit::Aggregate(items), _) => items
            .into_iter()
            .flat_map(|item| convert_static_init(item, OperandType::Int))
            .collect(),
        (crate::ir::tacky::TackyStaticInit::Int(n), OperandType::Long | OperandType::ULong) => {
            vec![StaticInit::Long(n)]
        }
        (crate::ir::tacky::TackyStaticInit::Int(n), OperandType::Byte | OperandType::UByte) => {
            vec![StaticInit::Char(n as u8)]
        }
        (crate::ir::tacky::TackyStaticInit::Int(n), _) => vec![StaticInit::Int(n)],
        (crate::ir::tacky::TackyStaticInit::Zero, OperandType::Long | OperandType::ULong) => {
            vec![StaticInit::Zero(8)]
        }
        (crate::ir::tacky::TackyStaticInit::Zero, OperandType::Double) => vec![StaticInit::Zero(8)],
        (crate::ir::tacky::TackyStaticInit::Zero, OperandType::ByteArray { size }) => {
            vec![StaticInit::Zero(size as u32)]
        }
        (crate::ir::tacky::TackyStaticInit::Zero, OperandType::Byte | OperandType::UByte) => {
            vec![StaticInit::Zero(1)]
        }
        (crate::ir::tacky::TackyStaticInit::Zero, _) => vec![StaticInit::Zero(4)],
        (crate::ir::tacky::TackyStaticInit::Long(n), OperandType::Byte | OperandType::UByte) => {
            vec![StaticInit::Char(n as u8)]
        }
        (crate::ir::tacky::TackyStaticInit::Long(n), _) => vec![StaticInit::Long(n)],
        (crate::ir::tacky::TackyStaticInit::Double(d), _) => vec![StaticInit::Double(d)],
        (crate::ir::tacky::TackyStaticInit::Char(c), _) => vec![StaticInit::Char(c)],
        (crate::ir::tacky::TackyStaticInit::StringBytes(bytes), _) => {
            vec![StaticInit::StringBytes(bytes)]
        }
        (crate::ir::tacky::TackyStaticInit::Pointer(label), _) => vec![StaticInit::Pointer(label)],
    }
}

pub fn generate(tacky: &TackyProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    let mut top_level: Vec<TopLevel> = Vec::new();
    let mut ctx = CodegenCtx {
        function_param_types: tacky.function_param_types.clone(),
        function_return_types: tacky.function_return_types.clone(),
        ..Default::default()
    };
    for var in &tacky.static_variables {
        let init = convert_static_init(var.init.clone(), var.ty);
        let alignment = match var.ty {
            OperandType::Long | OperandType::ULong | OperandType::Double => 8,
            OperandType::ByteArray { .. } => 16,
            OperandType::Byte | OperandType::UByte => 1,
            _ => 4,
        };
        top_level.push(TopLevel::StaticVariable {
            name: var.name.clone(),
            global: var.global,
            alignment,
            init,
        });
    }
    for constant in &tacky.static_constants {
        top_level.push(TopLevel::Constant {
            label: constant.name.clone(),
            value: constant.bytes.clone(),
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
        top_level.push(generate_function(func, &globals_type_env, &mut ctx));
    }
    for (label, value) in ctx.double_constants {
        top_level.push(TopLevel::Constant {
            label,
            value: value.to_bits().to_le_bytes().to_vec(),
        });
    }
    Ok(AsmProgram { top_level })
}
