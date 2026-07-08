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
// Chapter 3 adds the binary arithmetic, bitwise, and shift forms.
// Shift instructions need the count operand in `%cl` (the low byte of
// `%ecx`); the codegen pass emits the count via a placeholder
// `Reg::CX` operand and the emitter rewrites that placeholder to
// `%cl` for `BitShiftLeft` / `BitShiftRight` only — every other
// instruction keeps the standard `Reg::CX -> %ecx` mapping.  This is
// the smallest workaround that lets the shift tests run without
// extending `assembly.rs`'s register enum.
//
// Chapter 9 adds the `call` and `push`/`pop` forms plus the
// stack-alignment `AllocateStack` / `DeallocateStack` quartet
// (mirroring OCaml `emit.ml`'s `Push`/`Pop` arms).  The prologue /
// epilogue is now also emitted by the fixup pass (chapter 9), so
// `format_function` only writes the opening label and the per-
// instruction body; the fixup pass has already prepended the
// `AllocateStack` + callee-saved push sequence and appended the
// matching pop sequence + `ret`.
//
// Indentation is four spaces per the OCaml `emit_instruction` preamble
// (`\tmov%s %s, %s\n`).  Each line joins with `\n` and the program ends
// with a trailing newline so the file is line-terminated like every
// other hand-written or book-emitted `.s` source.

use anyhow::{Result, anyhow};

use crate::codegen::assembly::{
    AsmProgram, BinaryOpInstr, ConditionCode, Instr, Operand, Reg, TopLevel, UnaryOpInstr,
};

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

/// Quadword (64-bit) register name.  Used by `pushq` / `popq` and
/// `movq` so the ABI-sized register width is selected over the
/// 32-bit `%edi`/`%esi`/etc. that the integer ALU uses.
fn reg_name_quad(reg: &Reg) -> &'static str {
    match reg {
        Reg::AX => "%rax",
        Reg::CX => "%rcx",
        Reg::DX => "%rdx",
        Reg::DI => "%rdi",
        Reg::SI => "%rsi",
        Reg::R8 => "%r8",
        Reg::R9 => "%r9",
        Reg::R10 => "%r10",
        Reg::R11 => "%r11",
        Reg::SP => "%rsp",
        Reg::BP => "%rbp",
        Reg::BX => "%rbx",
        Reg::R12 => "%r12",
        Reg::R13 => "%r13",
        Reg::R14 => "%r14",
        Reg::R15 => "%r15",
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
        Operand::Memory(base, offset) => Ok(format!("{}({})", offset, reg_name_quad(base))),
        Operand::MemoryIndexed(base, index, scale) => Ok(format!(
            "({},{},{})",
            reg_name_quad(base),
            reg_name_quad(index),
            scale
        )),
        Operand::Stack(offset) => Ok(format!("{}(%rbp)", offset)),
        Operand::Data(name) => Ok(format!("{name}(%rip)")),
        Operand::Pseudo(name) => Err(anyhow!(
            "pseudoregister leaked past replace_pseudos: {name}"
        )),
    }
}

/// Like [`format_operand`] but uses the quadword register name; used by
/// `pushq` / `popq` and `movq` so the 64-bit register is selected.
fn format_quad_operand(op: &Operand) -> Result<String> {
    match op {
        Operand::Imm(n) => Ok(format!("${n}")),
        Operand::Reg(reg) => Ok(reg_name_quad(reg).to_string()),
        Operand::Memory(base, offset) => Ok(format!("{}({})", offset, reg_name_quad(base))),
        Operand::MemoryIndexed(base, index, scale) => Ok(format!(
            "({},{},{})",
            reg_name_quad(base),
            reg_name_quad(index),
            scale
        )),
        Operand::Stack(offset) => Ok(format!("{}(%rbp)", offset)),
        Operand::Data(name) => Ok(format!("{name}(%rip)")),
        Operand::Pseudo(name) => Err(anyhow!(
            "pseudoregister leaked past replace_pseudos: {name}"
        )),
    }
}

fn format_shift_src(op: BinaryOpInstr, src: &Operand) -> Result<String> {
    // x86-64 shift instructions accept only `%cl` (the low byte of `%ecx`)
    // as the count operand.  The codegen pass encodes the count via a
    // placeholder `Operand::Reg(Reg::CX)`; rewrite that placeholder to
    // `%cl` here so the emitted instruction is assembler-valid.
    match (op, src) {
        (BinaryOpInstr::BitShiftLeft | BinaryOpInstr::BitShiftRight, Operand::Reg(Reg::CX)) => {
            Ok("%cl".to_string())
        }
        _ => format_operand(src),
    }
}

/// Like [`format_shift_src`] but uses the quadword register-name
/// table; used by 64-bit binary ops to pick `%rcx` instead of
/// `%ecx` for the shift count (and `%cl` instead of `%cl` for the
/// count — both 32- and 64-bit shifts use the same `%cl`).
fn format_shift_quad(op: BinaryOpInstr, src: &Operand) -> Result<String> {
    match (op, src) {
        (BinaryOpInstr::BitShiftLeft | BinaryOpInstr::BitShiftRight, Operand::Reg(Reg::CX)) => {
            Ok("%cl".to_string())
        }
        _ => format_quad_operand(src),
    }
}

fn format_unary_op(op: UnaryOpInstr) -> &'static str {
    match op {
        UnaryOpInstr::Neg => "negl",
        UnaryOpInstr::Not => "notl",
        UnaryOpInstr::Shr => "shrl",
    }
}

fn format_cond_code(cc: ConditionCode) -> &'static str {
    match cc {
        ConditionCode::E => "e",
        ConditionCode::NE => "ne",
        ConditionCode::L => "l",
        ConditionCode::LE => "le",
        ConditionCode::G => "g",
        ConditionCode::GE => "ge",
        ConditionCode::A => "a",
        ConditionCode::AE => "ae",
        ConditionCode::B => "b",
        ConditionCode::BE => "be",
        ConditionCode::P => "p",
    }
}

fn format_binary_op(op: BinaryOpInstr) -> &'static str {
    match op {
        BinaryOpInstr::Add => "addl",
        BinaryOpInstr::Sub => "subl",
        BinaryOpInstr::Mult => "imull",
        BinaryOpInstr::DivDouble => "divl",
        BinaryOpInstr::DivSigned => "idivl",
        BinaryOpInstr::RemSigned => "idivl",
        BinaryOpInstr::BitAnd => "andl",
        BinaryOpInstr::BitOr => "orl",
        BinaryOpInstr::BitXor => "xorl",
        BinaryOpInstr::BitShiftLeft => "sall",
        BinaryOpInstr::BitShiftRight => "sarl",
        BinaryOpInstr::AddQ => "addq",
        BinaryOpInstr::SubQ => "subq",
        BinaryOpInstr::MultQ => "imulq",
        BinaryOpInstr::DivQ => "idivq",
        BinaryOpInstr::RemQ => "idivq",
        BinaryOpInstr::AddDouble => "addsd",
        BinaryOpInstr::SubDouble => "subsd",
        BinaryOpInstr::MultDouble => "mulsd",
        BinaryOpInstr::SseDivDouble => "divsd",
        BinaryOpInstr::XorDouble => "xorpd",
    }
}

/// Returns true when the binary op is a 64-bit variant
/// (`addq` / `subq` / `imulq` / `idivq`).  Used by the emitter to
/// pick the quadword register-name table for the operands.
fn is_wide_binary_op(op: BinaryOpInstr) -> bool {
    matches!(
        op,
        BinaryOpInstr::AddQ
            | BinaryOpInstr::SubQ
            | BinaryOpInstr::MultQ
            | BinaryOpInstr::DivQ
            | BinaryOpInstr::RemQ
    )
}

fn format_instruction(instr: &Instr) -> Result<String> {
    match instr {
        Instr::Mov { src, dst } => Ok(format!(
            "movl {}, {}",
            format_operand(src)?,
            format_operand(dst)?
        )),
        Instr::Movq { src, dst } => Ok(format!(
            "movq {}, {}",
            format_quad_operand(src)?,
            format_quad_operand(dst)?
        )),
        Instr::Movabsq { src, dst } => Ok(format!(
            "movabsq ${}, {}",
            src,
            format_quad_operand(dst)?
        )),
        Instr::MovZeroExtend { src, dst } => Ok(format!(
            "movzbl {}, {}",
            format_operand(src)?,
            format_operand(dst)?
        )),
Instr::Movsx { src, dst } => Ok(format!(
            "movslq {}, {}",
            format_operand(src)?,
            format_quad_operand(dst)?
        )),
        Instr::Unary { op, operand } => Ok(format!(
            "{} {}",
            format_unary_op(*op),
            format_operand(operand)?
        )),
        Instr::UnaryQ { op, operand } => Ok(format!(
            "{}q {}",
            format_unary_op(*op).trim_end_matches('l'),
            format_quad_operand(operand)?
        )),
        Instr::BinaryOp { op, src, dst } => {
            // 64-bit binary ops (AddQ/SubQ/MultQ/DivQ/RemQ) use
            // the quadword register-name table for the operands;
            // 32-bit ops use the longword table.
            let (src_str, dst_str) = if is_wide_binary_op(*op) {
                (
                    format_shift_quad(*op, src)?,
                    format_quad_operand(dst)?,
                )
            } else {
                (
                    format_shift_src(*op, src)?,
                    format_operand(dst)?,
                )
            };
            Ok(format!("{} {}, {}", format_binary_op(*op), src_str, dst_str))
        }
        Instr::Idiv(src) => Ok(format!("idivl {}", format_operand(src)?)),
        Instr::Idivq(src) => Ok(format!("idivq {}", format_quad_operand(src)?)),
        Instr::Cdq => Ok("cdq".to_string()),
        Instr::Cqo => Ok("cqo".to_string()),
        Instr::Cltq => Ok("cltq".to_string()),
        Instr::AllocateStack(n) => Ok(format!("subq ${n}, %rsp")),
        Instr::DeallocateStack(n) => Ok(format!("addq ${n}, %rsp")),
        Instr::Push(src) => Ok(format!("pushq {}", format_quad_operand(src)?)),
        Instr::Pop(reg) => Ok(format!("popq {}", reg_name_quad(reg))),
        Instr::Call(name) => Ok(format!("call {name}")),
        Instr::Ret => Ok("movq %rbp, %rsp\npopq %rbp\nret".to_string()),
        Instr::Cmp { left, right } => Ok(format!(
            "cmpl {}, {}",
            format_operand(right)?,
            format_operand(left)?
        )),
        Instr::Cmpq { left, right } => Ok(format!(
            "cmpq {}, {}",
            format_quad_operand(right)?,
            format_quad_operand(left)?
        )),
        Instr::Jmp(label) => Ok(format!("jmp {label}")),
        Instr::JmpCC { cc, label } => Ok(format!("j{} {label}", format_cond_code(*cc))),
        Instr::SetCC { cc, dst } => Ok(format!(
            "set{} {}",
            format_cond_code(*cc),
            format_operand(dst)?
        )),
        Instr::Label(name) => Ok(format!("{name}:")),
        other => Err(anyhow!(
            "emit does not yet support instruction variant: {other:?}"
        )),
    }
}

/// Emit a function prologue and per-instruction body.  The fixup
/// pass is expected to have already prepended the stack-allocation /
/// callee-saved push sequence and to append the matching pops + ret;
/// here we just write the function label and the per-instruction
/// body.  Chapter 9 fixup runs before the emitter, so this function
/// is intentionally minimal.
fn format_function(name: &str, global: bool, instructions: &[Instr]) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();
    // Switch back to .text after any .bss/.data block the static-
    // variable emitter may have left in scope.  Mirrors the OCaml
    // `emit_tl` `Function` arm.
    lines.push(".text".to_string());
    if global {
        lines.push(format!(".globl {name}"));
    }
    lines.push(format!("{name}:"));
    // The standard System V prologue: save %rbp, set up frame
    // pointer.  The actual stack-slot allocation happens via
    // `AllocateStack` emitted by the fixup pass.
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
                type_env: _,
            } => format_function(name, *global, instructions)?,
            TopLevel::StaticVariable {
                name,
                global,
                alignment,
                init,
            } => format_static_variable(name, *global, *alignment, init)?,
            TopLevel::Constant { label, value } => format_constant(label, value),
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

/// Emit a `.data` / `.bss` block for a static variable.  Mirrors
/// `emit_tl` `StaticVariable` arms in `nqcc2/lib/emit.ml:314-336`:
/// zero initializers go to `.bss` (the linker reserves space without
/// baking it into the executable image); everything else goes to
/// `.data`.  Alignment is emitted via `.align` (Linux AT&T syntax).
fn format_static_variable(
    name: &str,
    global: bool,
    alignment: u32,
    init: &crate::codegen::assembly::StaticInit,
) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();
    if global {
        lines.push(format!(".globl {name}"));
    }
    if is_zero_init(init) {
        lines.push(".bss".to_string());
        lines.push(format!(".align {alignment}"));
        lines.push(format!("{name}:"));
        lines.push(format!("    .zero {}", zero_size(init)));
    } else {
        lines.push(".data".to_string());
        lines.push(format!(".align {alignment}"));
        lines.push(format!("{name}:"));
        // Chapter 11: 64-bit statics use `.quad` so the linker
        // reserves 8 bytes; 32-bit statics keep `.long` (4 bytes).
        match init {
            crate::codegen::assembly::StaticInit::Long(n) => {
                lines.push(format!("    .quad {n}"));
            }
            _ => {
                lines.push(format!("    .long {}", data_value(init)?));
            }
        }
    }
    Ok(lines.join("\n"))
}

fn is_zero_init(init: &crate::codegen::assembly::StaticInit) -> bool {
    use crate::codegen::assembly::StaticInit;
    matches!(
        init,
        StaticInit::Int(0) | StaticInit::Long(0) | StaticInit::Zero(_)
    )
}

fn zero_size(init: &crate::codegen::assembly::StaticInit) -> u32 {
    use crate::codegen::assembly::StaticInit;
    match init {
        StaticInit::Zero(n) => *n,
        StaticInit::Long(_) => 8,
        StaticInit::Int(_) => 4,
        _ => 4,
    }
}

fn data_value(init: &crate::codegen::assembly::StaticInit) -> Result<i64> {
    use crate::codegen::assembly::StaticInit;
    match init {
        StaticInit::Int(n) => Ok(*n),
        StaticInit::Long(n) => Ok(*n),
        other => Err(anyhow!(
            "emit chapter-10 does not yet emit non-int static initializer: {other:?}"
        )),
    }
}

/// Emit a `.rodata`-style constant pool entry.  Mirrors the
/// `StaticConstant` arm of `nqcc2/lib/emit.ml`.
fn format_constant(label: &str, value: &[u8]) -> String {
    let bytes = value
        .iter()
        .map(|b| format!("{b}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(".section .rodata\n.align 4\n{label}:\n    .byte {bytes}")
}