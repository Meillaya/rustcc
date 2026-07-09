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

use crate::codegen::assembly::{AsmProgram, Instr, Operand, TopLevel};

mod move_split;
mod split;

use crate::ir::tacky::{OperandType, TypeEnv};
use split::split_memory_to_memory;

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
        let ty = self.type_env.get(name).copied().unwrap_or(OperandType::Int);
        let size = ty.size() as i32;
        let alignment = match ty {
            OperandType::ByteArray { size } if size >= 16 => 16,
            OperandType::Long | OperandType::ULong | OperandType::Double => 8,
            _ => 4,
        };
        let offset = *self.pseudos.entry(name.to_string()).or_insert_with(|| {
            let needed = self.stack_size + size;
            let aligned = ((needed + alignment - 1) / alignment) * alignment;
            self.stack_size = aligned;
            -aligned
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
        Operand::PseudoMem(name, offset) => {
            if globals.contains(&name) {
                Operand::DataOffset(name, offset)
            } else {
                match state.resolve(&name) {
                    Operand::Stack(base) => Operand::Stack(base + offset),
                    other => other,
                }
            }
        }
        other => other,
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
        Instr::MovByte { src, dst } => Instr::MovByte {
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
        Instr::MovSignExtendByte { src, dst } => Instr::MovSignExtendByte {
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
                let prologue = (aligned_size > 0).then_some(Instr::AllocateStack(aligned_size));
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
