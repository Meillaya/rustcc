// Mirrors nqcc2/lib/tacky.ml (99 LOC) and nqcc2/lib/tacky_gen.ml (593 LOC). Locked to book-faithful TACKY IR.
//
// The book grows this IR chapter by chapter; the full enum surface is declared
// now so subsequent waves can land codegen, optimization, and floating-point
// variants without disturbing downstream modules.  Variants that the current
// wave does not consume (e.g. floating-point conversions, bitwise shifts) stay
// live because `#[allow(dead_code)]` is module-level; this keeps rustc happy
// while preserving the mirror.

#![allow(dead_code)]

use anyhow::Result;

use crate::semantics::typecheck::TypedProgram;

/// A TACKY value: either an inline integer constant or a named variable.
///
/// Mirrors `nqcc2/lib/tacky.ml` `tacky_val`.  Chapter 13 introduces a
/// `ConstantDouble(f64)` arm; left as a comment because the book's dial-in
/// starts in chapter 3 with `Constant(i64)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Val {
    Constant(i64),
    Var(String),
    // ch.13: ConstantDouble(f64),
}

/// x86-64 condition codes used by `Cmp` (which captures both branches) and
/// the auxiliary `JumpIfZero` / `JumpIfNotZero` instructions that use the
/// implied "compare against zero" form.
///
/// Mirrors `nqcc2/lib/assembly.ml` `condition_code`.  Chapter 4 uses the
/// signed comparison codes (`E`, `NE`, `L`, `LE`, `G`, `GE`); the unsigned
/// codes (`A`, `AE`, `B`, `BE`) and the parity code (`P`) are reserved
/// for the chapter-12 unsigned work.
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

/// A TACKY pseudo-variable.  Defined as a type alias to keep the surface flat
/// and to match OCaml's pervasive `string` Var representation.
pub type Var = String;

/// A TACKY instruction: the closed set of operations the codegen pass lowers.
///
/// Mirrors `nqcc2/lib/tacky.ml` `instruction`.  Variants are intentionally
/// spelled to match the OCaml AST one-for-one (e.g. `BitShiftLeft` not
/// `Shl`); the book grows this list incrementally across chapters 3-13.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Instruction {
    Return(Val),
    SignExtend {
        src: Val,
        dst: Var,
    },
    ZeroExtend {
        src: Val,
        dst: Var,
    },
    Truncate {
        src: Val,
        dst: Var,
    },
    IntToDouble {
        src: Val,
        dst: Var,
    },
    DoubleToInt {
        src: Val,
        dst: Var,
    },
    UIntToDouble {
        src: Val,
        dst: Var,
    },
    DoubleToUInt {
        src: Val,
        dst: Var,
    },
    Add {
        src: Val,
        dst: Var,
    },
    Sub {
        src: Val,
        dst: Var,
    },
    Mul {
        src: Val,
        dst: Var,
    },
    DivSigned {
        src: Val,
        dst: Var,
    },
    RemSigned {
        src: Val,
        dst: Var,
    },
    BitAnd {
        src: Val,
        dst: Var,
    },
    BitOr {
        src: Val,
        dst: Var,
    },
    BitXor {
        src: Val,
        dst: Var,
    },
    BitShiftLeft {
        src: Val,
        dst: Var,
    },
    BitShiftRight {
        src: Val,
        dst: Var,
    },
    Negate {
        dst: Var,
    },
    Complement {
        dst: Var,
    },
    Not {
        dst: Var,
    },
    /// Chapter 4 comparison instruction.  Lowerer emits this for the
    /// equality / relational operators and for the unary `Not`
    /// operator; codegen turns it into a `cmpl` + `setCC` + `movzbl`
    /// trio so the destination holds the 0/1 boolean result.
    ///
    /// The compare uses x86-64 `cmpl right, left` semantics (AT&T:
    /// the second operand is the source of the subtraction): flags
    /// are set as if computing `left - right`.  The condition code
    /// `cc` selects which boolean predicate the comparison satisfies.
    /// Mirrors `nqcc2/lib/tacky_gen.ml` `Equal | NotEqual |
    /// GreaterThan | GreaterOrEqual | LessThan | LessOrEqual` arms.
    Cmp {
        left: Val,
        right: Val,
        dst: Var,
        cc: ConditionCode,
    },
    Jump {
        target: String,
    },
    JumpIfZero {
        condition: Val,
        target: String,
    },
    JumpIfNotZero {
        condition: Val,
        target: String,
    },
    Label(String),
    Copy {
        src: Val,
        dst: Var,
    },
    Load {
        src_pointer: Val,
        dst: Var,
    },
    Store {
        src: Val,
        dst_pointer: Val,
    },
    GetAddress {
        src: Var,
        dst: Var,
    },
    AddPtr {
        ptr: Val,
        index: Val,
        scale: u8,
        dst: Var,
    },
    Call {
        name: String,
        args: Vec<Val>,
        dst: Option<Var>,
    },
}

/// A TACKY function: a name, parameter names, and a flat list of instructions.
///
/// Mirrors `nqcc2/lib/tacky.ml` `Function { name; global; params; body }`.
/// Chapter 9 widens this with `params: Vec<String>` so the codegen pass
/// can emit the prologue that moves each parameter from its incoming
/// register to the function's stack slot.  Earlier chapters left
/// `params` implicit (the function had no parameters).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TackyFunction {
    pub name: String,
    pub global: bool,
    pub params: Vec<String>,
    pub body: Vec<Instruction>,
}

/// A TACKY program: a list of functions.
///
/// Mirrors `nqcc2/lib/tacky.ml` `program`.  The single-function main of
/// chapters 1-7 fits in a `vec![one_entry]`; multi-function support lands in
/// chapters 9+ alongside the chapter-9 function declarations.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TackyProgram {
    pub functions: Vec<TackyFunction>,
}

/// Lower the typed AST into a TACKY program.
///
/// Thin wrapper over [`crate::ir::lower::lower_program`] so
/// `src/ir/tacky.rs` remains the public entry point of the TACKY IR
/// surface (per the W0-T6 plan).  The real lowering walks each top-level
/// declaration, allocates temporaries via `TempIdGenerator`, and emits a
/// flat `Vec<Instruction>` per function.  This delegation keeps the
/// `&TypedProgram` parameter consistent with the rest of the pipeline
/// (`&semantics::typecheck::TypedProgram`) while the lowering itself
/// consumes the inner AST shape via `ast.program`.
pub fn ast_to_tacky(ast: &TypedProgram) -> Result<TackyProgram> {
    crate::ir::lower::lower_program(&ast.program)
}
