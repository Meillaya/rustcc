// Mirrors nqcc2/lib/backend/replace_pseudos.ml plus the chapter-2 subset
// of `nqcc2/lib/backend/instruction_fixup.ml`. The OCaml pass rewrites
// every `Pseudo(name)` operand into either a real register (one of the
// callee-saved set chosen by the allocator), a `Stack(offset)` slot
// relative to `%rbp`, or a `Data(name)` RIP-relative reference for
// chapter-10 file-scope variables. The chapter-2 fixup then (a)
// prepends `AllocateStack(bytes_used)` so the frame actually claims the
// stack space, and (b) splits `Mov(Stack, Stack)` into two moves via a
// scratch register because x86-64 does not allow memory-to-memory
// `movl`.

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::codegen::assembly::{AsmProgram, BinaryOpInstr, Instr, Operand, Reg, TopLevel};
use crate::ir::tacky::{OperandType, TypeEnv};

struct ReplaceState {
    stack_size: i32,
    pseudos: HashMap<String, i32>,
    type_env: TypeEnv,
}

impl ReplaceState {
    fn new(type_env: &TypeEnv) -> Self {
        Self {
            stack_size: 0,
            pseudos: HashMap::new(),
            type_env: type_env.clone(),
        }
    }

    fn resolve(&mut self, name: &str) -> Operand {
        let size = self
            .type_env
            .get(name)
            .map(|t| t.size())
            .unwrap_or(OperandType::Int.size()) as i32;
        let offset = *self.pseudos.entry(name.to_string()).or_insert_with(|| {
            let current = -(self.stack_size + size);
            self.stack_size += size;
            current
        });
        Operand::Stack(offset)
    }
}

fn replace_operand(state: &mut ReplaceState, op: Operand, globals: &HashSet<String>) -> Operand {
    match op {
        Operand::Pseudo(name) => {
            if globals.contains(&name) {
                Operand::Data(name)
            } else {
                state.resolve(&name)
            }
        }
        other => other,
    }
}

fn split_memory_to_memory(instr: Instr) -> Vec<Instr> {
    match instr {
        // Chapter 10: route every memory-to-memory `movl` through
        // `%r10d` so `mov src, dst` works when both operands are
        // stack slots or RIP-relative data references.
        Instr::Mov {
            src: src @ (Operand::Stack(_) | Operand::Data(_)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
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
        // Chapter 11: same split for the 64-bit `movq`.
        Instr::Movq {
            src: src @ (Operand::Stack(_) | Operand::Data(_)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
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
            src: src @ (Operand::Stack(_) | Operand::Data(_)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
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
        // Chapter 11: `movslq` requires a register destination
        // (x86-64 forbids memory destinations for sign-extending
        // moves).  Route through `%r10` whenever the destination is
        // a stack slot or a RIP-relative data reference.
        Instr::Movsx {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Movsx {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Lea {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Lea {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::BinaryOp {
            op,
            src: src @ (Operand::Stack(_) | Operand::Data(_)),
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
        } => {
            // Chapter 11: 64-bit binary ops (AddQ, SubQ, MultQ,
            // BitAnd as longword, etc.) need a 64-bit move to the
            // scratch register, not the default 32-bit `movl`.  We
            // route the wide class through `Movq` and the narrow
            // class through `Mov`.
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
        Instr::Idiv(src @ (Operand::Stack(_) | Operand::Data(_))) => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Idiv(Operand::Reg(Reg::R10)),
        ],
        Instr::Div(src @ (Operand::Stack(_) | Operand::Data(_))) => vec![
            Instr::Mov {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Div(Operand::Reg(Reg::R10)),
        ],
        Instr::Idivq(src @ (Operand::Stack(_) | Operand::Data(_))) => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Idivq(Operand::Reg(Reg::R10)),
        ],
        Instr::Divq(src @ (Operand::Stack(_) | Operand::Data(_))) => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Divq(Operand::Reg(Reg::R10)),
        ],
        // Chapter 4 + 10: `cmpl mem, mem` is invalid; route the
        // right operand through a scratch register.
        Instr::Cmp {
            left,
            right: right @ (Operand::Stack(_) | Operand::Data(_)),
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
        // Chapter 11: same split for the 64-bit `cmpq`.
        Instr::Cmpq {
            left,
            right: right @ (Operand::Stack(_) | Operand::Data(_)),
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
            right: right @ (Operand::Stack(_) | Operand::Data(_)),
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
        Instr::Cvttsd2si {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Cvttsd2si {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Movq {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Cvtsi2sd {
            src: src @ Operand::Imm(_),
            dst: dst @ Operand::Reg(_),
        } => vec![
            Instr::Movq {
                src,
                dst: Operand::Reg(Reg::R10),
            },
            Instr::Cvtsi2sd {
                src: Operand::Reg(Reg::R10),
                dst,
            },
        ],
        Instr::Cvtsi2sd {
            src,
            dst: dst @ (Operand::Stack(_) | Operand::Data(_)),
        } => vec![
            Instr::Cvtsi2sd {
                src,
                dst: Operand::Reg(Reg::XMM(15)),
            },
            Instr::Movsd {
                src: Operand::Reg(Reg::XMM(15)),
                dst,
            },
        ],
        other => vec![other],
    }
}

fn replace_in_instruction(
    state: &mut ReplaceState,
    instr: Instr,
    globals: &HashSet<String>,
) -> Vec<Instr> {
    let instr = match instr {
        Instr::Mov { src, dst } => Instr::Mov {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Movq { src, dst } => Instr::Movq {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Movabsq { src, dst } => Instr::Movabsq {
            src,
            dst: replace_operand(state, dst, globals),
        },
        Instr::MovZeroExtend { src, dst } => Instr::MovZeroExtend {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Unary { op, operand } => Instr::Unary {
            op,
            operand: replace_operand(state, operand, globals),
        },
        Instr::UnaryQ { op, operand } => Instr::UnaryQ {
            op,
            operand: replace_operand(state, operand, globals),
        },
        Instr::BinaryOp { op, src, dst } => Instr::BinaryOp {
            op,
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Idiv(src) => Instr::Idiv(replace_operand(state, src, globals)),
        Instr::Div(src) => Instr::Div(replace_operand(state, src, globals)),
        Instr::Idivq(src) => Instr::Idivq(replace_operand(state, src, globals)),
        Instr::Divq(src) => Instr::Divq(replace_operand(state, src, globals)),
        Instr::Cmp { left, right } => Instr::Cmp {
            left: replace_operand(state, left, globals),
            right: replace_operand(state, right, globals),
        },
        Instr::Cmpq { left, right } => Instr::Cmpq {
            left: replace_operand(state, left, globals),
            right: replace_operand(state, right, globals),
        },
        Instr::SetCC { cc, dst } => Instr::SetCC {
            cc,
            dst: replace_operand(state, dst, globals),
        },
        Instr::Push(src) => Instr::Push(replace_operand(state, src, globals)),
        Instr::AllocateStack(_) => instr,
        Instr::DeallocateStack(_) => instr,
        Instr::Jmp(_) | Instr::JmpCC { .. } | Instr::Label(_) => instr,
        Instr::Call(_) | Instr::Pop(_) | Instr::Ret | Instr::Cdq | Instr::Cqo | Instr::Cltq => {
            instr
        }
        Instr::Movsx { src, dst } => Instr::Movsx {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Lea { src, dst } => Instr::Lea {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Movsd { src, dst } => Instr::Movsd {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::MovsdLoad { src, dst } => Instr::MovsdLoad {
            src,
            dst: replace_operand(state, dst, globals),
        },
        Instr::CmpDouble { left, right } => Instr::CmpDouble {
            left: replace_operand(state, left, globals),
            right: replace_operand(state, right, globals),
        },
        Instr::Cvtsi2sd { src, dst } => Instr::Cvtsi2sd {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Cvttsd2si { src, dst } => Instr::Cvttsd2si {
            src: replace_operand(state, src, globals),
            dst: replace_operand(state, dst, globals),
        },
        Instr::Comment(_) => instr,
    };
    split_memory_to_memory(instr)
}

/// Walk each top-level `Fn`'s instruction list and translate every
/// `Pseudo` operand into a `Stack(offset)` (for locals / parameters)
/// or a `Data(name)` (for file-scope globals).  Prepend an
/// `AllocateStack` that reserves the temporary area and split any
/// memory-to-memory `mov` into two scratch-register moves so the
/// output is assembler-valid.
pub fn replace_pseudos(asm: AsmProgram, globals: &HashSet<String>) -> Result<AsmProgram> {
    let AsmProgram { top_level } = asm;
    let top_level = top_level
        .into_iter()
        .map(|tl| match tl {
            TopLevel::Fn {
                name,
                global,
                instructions,
                type_env,
            } => {
                let mut state = ReplaceState::new(&type_env);
                let mut fixed: Vec<Instr> = Vec::new();
                for instr in instructions {
                    fixed.extend(replace_in_instruction(&mut state, instr, globals));
                }
                let raw_size = state.stack_size;
                let aligned_size = if raw_size == 0 {
                    0
                } else {
                    ((raw_size + 15) / 16) * 16
                };
                let prologue = (aligned_size > 0).then(|| Instr::AllocateStack(aligned_size));
                let mut ordered = Vec::with_capacity(fixed.len() + 1);
                if let Some(alloc) = prologue {
                    ordered.push(alloc);
                }
                ordered.extend(fixed);
                TopLevel::Fn {
                    name,
                    global,
                    instructions: ordered,
                    type_env,
                }
            }
            other => other,
        })
        .collect();
    Ok(AsmProgram { top_level })
}
