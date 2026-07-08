// Mirrors nqcc2/lib/assembly.ml (129 LOC). Locked to x86-64 AT&T syntax, System V AMD64 ABI.
//
// This file is the type system only: it defines the assembly AST that the
// codegen pass produces and the emitter pass consumes. No codegen or printing
// logic lives here — those arrive in W0-T5 / W10. The variant set tracks the
// book chapters (1 through 13), so variants for SSE doubles (`XMM(u8)`,
// `Double` / `DivDouble` / `AddDouble` / etc.) and chapter-10 statics
// (`StaticInit`, `Constant`) are present even though they are not consumed yet.
#![allow(dead_code)]

/// A physical x86-64 register. `XMM(n)` covers XMM0..XMM15 used in
/// chapter 13 (doubles via System V's XMM parameter passing convention).
///
/// The order of unit variants is fixed by `Reg::encode` / `Reg::slot`
/// requirements in later waves — do not reorder without checking the
/// register allocator and frame layout assumptions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Reg {
    AX,
    CX,
    DX,
    DI,
    SI,
    R8,
    R9,
    R10,
    R11,
    SP,
    BP,
    BX,
    R12,
    R13,
    R14,
    R15,
    /// XMM0..XMM15. The wrapped index is in `0..=15`.
    XMM(u8),
}

/// An assembly operand. Mirrors `nqcc2/lib/assembly.ml`:
/// - `Imm`: an integer literal
/// - `Reg`: a physical register
/// - `Memory(base, offset)`: `[base + offset]`
/// - `MemoryIndexed(base, index, scale)`: `[base + index*scale + 0]`
///   (scale is the 1/2/4/8 SIB selector; the displacement stays in `Memory`
///   for the book, but the Rust port collapses the OCaml `Indexed` record
///   into a tuple to keep the enum flat)
/// - `Pseudo`: a symbolic operand the replace-pseudos pass will resolve
/// - `Stack`: a frame-relative slot (offset in bytes from `%rbp`)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    Imm(i64),
    Reg(Reg),
    Memory(Reg, i32),
    MemoryIndexed(Reg, Reg, i32),
    Pseudo(String),
    Stack(i32),
    /// RIP-relative reference to a file-scope static variable;
    /// emitted as `name(%rip)` in AT&T syntax.
    Data(String),
}

/// The arithmetic / logical operator carried by `Instr::BinaryOp`. Naming
/// follows the book: `DivDouble` is integer division that needs a CDQ
/// setup; `DivSigned` / `RemSigned` arrive in chapter 11; the `*Double`
/// variants are chapter 13.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOpInstr {
    Add,
    Sub,
    Mult,
    DivDouble,
    DivSigned,
    RemSigned,
    BitAnd,
    BitOr,
    BitXor,
    BitShiftLeft,
    BitShiftRight,
    /// Chapter 11: 64-bit counterparts.  Emitted as `addq`, `subq`,
    /// `imulq`, `idivq` (the latter paired with `cqo` to sign-extend
    /// %rax into %rdx).  The 32-bit forms above are preserved for
    /// the int surface.
    AddQ,
    SubQ,
    MultQ,
    DivQ,
    RemQ,
    AddDouble,
    SubDouble,
    MultDouble,
    DivDoubleDouble,
}

/// The unary operator carried by `Instr::Unary`. Mirrors
/// `nqcc2/lib/assembly.ml` `unary_operator`. Chapter 2 covers `Neg`
/// (`negl`) and `Not` (`notl`); `Shr` arrives in chapter 19.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOpInstr {
    Neg,
    Not,
    Shr,
}

/// x86-64 condition codes used by `JmpCC` and `SetCC`. `P` (parity) is
/// included because `setCC` may be emitted by later chapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConditionCode {
    E,
    NE,
    L,
    LE,
    G,
    GE,
    A,
    AE,
    B,
    BE,
    P,
}

/// A single assembly instruction. The variant set is the union of every
/// instruction the book teaches through chapter 13; some are unused by the
/// current codegen (e.g. `Cvttsd2si`, `AllocateStack`) and will only be
/// emitted by future waves.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Instr {
    Mov {
        src: Operand,
        dst: Operand,
    },
    /// Chapter 11: 64-bit variant of `Mov`.  Emitted as `movq`; the
    /// emitter uses the 64-bit register names (`%rax` / `%rdi` /
    /// ...) when formatting the operands.
    Movq {
        src: Operand,
        dst: Operand,
    },
    /// Chapter 11: 64-bit immediate-to-register move for values
    /// that don't fit in a 32-bit sign-extended immediate
    /// (`movq imm32, mem` is only valid when the immediate fits in
    /// 32 bits).  Always lowered to a register destination and the
    /// `movabsq` mnemonic; the emitter prints `movq` for short
    /// immediates so this variant is the long-immediate escape
    /// hatch.  Mirrors the OCaml `Mov (Quadword, Imm, Reg)` arm in
    /// `nqcc2/lib/backend/codegen.ml`.
    Movabsq {
        src: i64,
        dst: Operand,
    },
    /// Sign-extending move (e.g. int -> long).  Emitted as `movslq`
    /// (sign-extend 32 -> 64 with sign extension).
    Movsx {
        src: Operand,
        dst: Operand,
    },
    /// Zero-extending move (e.g. unsigned int -> long).
    MovZeroExtend {
        src: Operand,
        dst: Operand,
    },
    Lea {
        src: Operand,
        dst: Operand,
    },
    Cmp {
        left: Operand,
        right: Operand,
    },
    /// Chapter 11: 64-bit variant of `Cmp`.  Emitted as `cmpq`.
    Cmpq {
        left: Operand,
        right: Operand,
    },
    BinaryOp {
        op: BinaryOpInstr,
        src: Operand,
        dst: Operand,
    },
    Idiv(Operand),
    /// Chapter 11: 64-bit variant of `Idiv`.  Pairs with `Cqo`.
    Idivq(Operand),
    Cdq,
    /// Chapter 11: sign-extend %rax into %rdx:%rax for 64-bit
    /// signed division.  Pairs with `Idivq`; the emitter prints
    /// `cqo`.
    Cqo,
    /// Chapter 11: sign-extend %eax into %rax (`cltq` / `cdqe`).
    /// Used when an int result in %eax participates in a 64-bit
    /// operation (e.g. the long-side operand of `idivq`).
    Cltq,
    Unary {
        op: UnaryOpInstr,
        operand: Operand,
    },
    /// Chapter 11: 64-bit counterpart of `Unary`.  Emitter prints
    /// `negq` / `notq` / `shrq` instead of the 32-bit forms.
    UnaryQ {
        op: UnaryOpInstr,
        operand: Operand,
    },
    Call(String),
    Ret,
    Push(Operand),
    Pop(Reg),
    Jmp(String),
    JmpCC {
        cc: ConditionCode,
        label: String,
    },
    SetCC {
        cc: ConditionCode,
        dst: Operand,
    },
    Label(String),
    /// Subtract `n` from `%rsp` to grow the stack frame.
    AllocateStack(i32),
    /// Add `n` back to `%rsp` to shrink the stack frame.
    DeallocateStack(i32),
    /// Inline comment emitted alongside the instruction stream.
    Comment(String),
}

/// Static initializers for chapter-10+ data sections. The OCaml
/// `Initializers` module has more variants (Char/UChar/etc.); the Rust
/// port collapses the same data into the variants below. `Double`
/// carries an `f64`, which is why `StaticInit` deliberately omits
/// `Eq`/`Hash` (floats are not `Eq` in Rust and deriving `Hash`
/// would require a manual impl).
#[derive(Clone, Debug, PartialEq)]
pub enum StaticInit {
    Int(i64),
    Long(i64),
    UInt(u64),
    ULong(u64),
    Double(f64),
    /// `n` zero bytes.
    Zero(u32),
    Char(u8),
    /// Raw string bytes (already escaped by the parser); the emitter
    /// appends the trailing NUL when needed.
    StringBytes(Vec<u8>),
    /// Address-of another static, e.g. `Pointer("x")` -> `&x` in the
    /// emitted `.data` section.
    Pointer(String),
}

/// A top-level assembly item. Mirrors `assembly.ml`'s `top_level`:
/// - `Fn`: a function body
/// - `StaticVariable`: a `.data` / `.bss` variable with initializers
/// - `Constant`: a `.rodata`-style constant the book emits for strings
///   and chapter-10 statics
#[derive(Clone, Debug, PartialEq)]
pub enum TopLevel {
    Fn {
        name: String,
        global: bool,
        instructions: Vec<Instr>,
        /// Chapter 11: per-function map from pseudo name to
        /// operand width (`Int` -> 4-byte slot, `Long` -> 8-byte
        /// slot).  Populated by the codegen pass and consumed by
        /// `replace_pseudos` to size the stack frame correctly.
        type_env: crate::ir::tacky::TypeEnv,
    },
    StaticVariable {
        name: String,
        global: bool,
        alignment: u32,
        init: StaticInit,
    },
    Constant {
        label: String,
        value: Vec<u8>,
    },
}

/// The complete assembly program: an ordered list of top-level items.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AsmProgram {
    pub top_level: Vec<TopLevel>,
}