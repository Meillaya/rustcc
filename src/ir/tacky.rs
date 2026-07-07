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

/// A TACKY function: a name and a flat list of instructions.
///
/// Mirrors `nqcc2/lib/tacky.ml` `function_definition`.  Book chapters keep this
/// record minimal until later chapters add prologue/epilogue and parameter
/// handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TackyFunction {
    pub name: String,
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
/// Mirrors `nqcc2/lib/tacky_gen.ml` (593 LOC).  The real implementation walks
/// each top-level declaration, allocates temporaries via `TempIdGenerator`,
/// and emits a flat `Vec<Instruction>` per function.  This stub keeps the
/// pipeline wired to chapter 1's `int main(void) { return N; }` shape; the
/// general lowering lands in W2-T2.
pub fn ast_to_tacky(_ast: &TypedProgram) -> Result<TackyProgram> {
    unimplemented!()
}
