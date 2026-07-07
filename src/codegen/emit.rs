// Mirrors nqcc2/lib/emit.ml.
//
// The OCaml emitter walks the `AsmProgram` and produces x86-64 AT&T
// assembly text.  Chapter 1 delivered the `movl $n, %eax` + `ret` pair;
// chapter 2 widens the surface:
//
//   .globl main
//   main:
//       pushq %rbp
//       movq %rsp, %rbp
//       subq $N, %rsp       (only when locals / temporaries are needed)
//       <chapter-2 body>    (movl / negl / notl between stack slots and %eax)
//       movq %rbp, %rsp
//       popq %rbp
//       ret
//
// Indentation is four spaces per the OCaml `emit_instruction` preamble
// (`\tmov%s %s, %s\n`).  Each line joins with `\n` and the program ends
// with a trailing newline so the file is line-terminated like every
// other hand-written or book-emitted `.s` source.

use anyhow::{Result, anyhow};

use crate::codegen::assembly::{AsmProgram, Instr, Operand, Reg, TopLevel, UnaryOpInstr};

const INDENT: &str = "    ";

fn reg_name(reg: &Reg) -> &'static str {
    match reg {
        Reg::AX => "%eax",
        Reg::CX => "%ecx",
        Reg::DX => "%edx",
        Reg::DI => "%edi",
        Reg::SI => "%esi",
        Reg::R8 => "%r8d",
        Reg::R9 => "%r9d",
        Reg::R10 => "%r10d",
        Reg::R11 => "%r11d",
        Reg::SP => "%esp",
        Reg::BP => "%ebp",
        Reg::BX => "%ebx",
        Reg::R12 => "%r12d",
        Reg::R13 => "%r13d",
        Reg::R14 => "%r14d",
        Reg::R15 => "%r15d",
        Reg::XMM(n) => match n {
            0 => "%xmm0",
            1 => "%xmm1",
            2 => "%xmm2",
            3 => "%xmm3",
            4 => "%xmm4",
            5 => "%xmm5",
            6 => "%xmm6",
            7 => "%xmm7",
            8 => "%xmm8",
            9 => "%xmm9",
            10 => "%xmm10",
            11 => "%xmm11",
            12 => "%xmm12",
            13 => "%xmm13",
            14 => "%xmm14",
            15 => "%xmm15",
            _ => "%xmm?",
        },
    }
}

fn format_operand(op: &Operand) -> Result<String> {
    match op {
        Operand::Imm(n) => Ok(format!("${n}")),
        Operand::Reg(reg) => Ok(reg_name(reg).to_string()),
        Operand::Memory(base, offset) => Ok(format!("{}({})", offset, reg_name(base))),
        Operand::MemoryIndexed(base, index, scale) => Ok(format!(
            "({},{},{})",
            reg_name(base),
            reg_name(index),
            scale
        )),
        Operand::Stack(offset) => Ok(format!("{}(%rbp)", offset)),
        Operand::Pseudo(_) => Err(anyhow!(
            "pseudoregister leaked past replace_pseudos (codegen regression)"
        )),
    }
}

fn format_unary_op(op: UnaryOpInstr) -> &'static str {
    match op {
        UnaryOpInstr::Neg => "negl",
        UnaryOpInstr::Not => "notl",
        UnaryOpInstr::Shr => "shrl",
    }
}

fn format_instruction(instr: &Instr) -> Result<String> {
    match instr {
        Instr::Mov { src, dst } => Ok(format!(
            "movl {}, {}",
            format_operand(src)?,
            format_operand(dst)?
        )),
        Instr::Unary { op, operand } => Ok(format!(
            "{} {}",
            format_unary_op(*op),
            format_operand(operand)?
        )),
        Instr::AllocateStack(n) => Ok(format!("subq ${n}, %rsp")),
        Instr::DeallocateStack(n) => Ok(format!("addq ${n}, %rsp")),
        Instr::Ret => Ok("movq %rbp, %rsp\npopq %rbp\nret".to_string()),
        other => Err(anyhow!(
            "ch.2 emit does not yet support instruction variant: {other:?}"
        )),
    }
}

fn format_function(name: &str, global: bool, instructions: &[Instr]) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();
    if global {
        lines.push(format!(".globl {name}"));
    }
    lines.push(format!("{name}:"));
    lines.push(format!("{INDENT}pushq %rbp"));
    lines.push(format!("{INDENT}movq %rsp, %rbp"));
    for instr in instructions {
        let rendered = format_instruction(instr)?;
        for line in rendered.lines() {
            lines.push(format!("{INDENT}{line}"));
        }
    }
    Ok(lines.join("\n"))
}

pub fn emit(program: &AsmProgram) -> Result<String> {
    let mut blocks: Vec<String> = Vec::new();
    for (idx, item) in program.top_level.iter().enumerate() {
        let block = match item {
            TopLevel::Fn {
                name,
                global,
                instructions,
            } => format_function(name, *global, instructions)?,
            TopLevel::StaticVariable { .. } | TopLevel::Constant { .. } => {
                return Err(anyhow!(
                    "ch.2 only emits Fn top-levels; data sections land in W12+"
                ));
            }
        };
        if idx == 0 {
            blocks.push(block);
        } else {
            blocks.push(format!("\n{block}"));
        }
    }
    blocks.push(String::new());
    Ok(blocks.join("\n"))
}
