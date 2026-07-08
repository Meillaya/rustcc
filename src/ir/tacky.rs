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

/// A TACKY value: either an inline integer/double constant or a named variable.
///
/// Mirrors `nqcc2/lib/tacky.ml` `tacky_val`.  `ConstantDouble(f64)` arrives
/// in chapter 13; because `f64` lacks `Eq`/`Hash`, this enum drops those
/// derivations even though its integer-only arms would otherwise support them.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Constant(i64),
    Var(String),
    ConstantDouble(f64),
}

/// Operand width / flavor for TACKY values.  Mirrors `nqcc2/lib/tacky.ml`
/// `asm_type` (Longword / Quadword / Double) for the chapter-13 surface.
/// The codegen pass uses this to choose between 32-bit and 64-bit x86-64
/// instructions (`addl` vs `addq`, `idivl` vs `idivq`, etc.) and to pick
/// the right SSE instruction for double values.
///
/// Chapter 12 widens the surface with `UInt` and `ULong`; the signedness
/// distinction is needed at compare-codegen time (unsigned `<` uses
/// `seta` rather than `setg`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperandType {
    /// 32-bit signed integer (book `Longword`).
    Int,
    /// 32-bit unsigned integer.
    UInt,
    Byte,
    UByte,
    /// 64-bit signed integer (book `Quadword`); used for `long` and pointers.
    Long,
    /// 64-bit unsigned integer.
    ULong,
    /// 64-bit IEEE-754 double (book `Double`).
    Double,
    ByteArray {
        size: i64,
    },
}

impl OperandType {
    /// Size in bytes — used by `replace_pseudos` to size stack slots.
    pub fn size(self) -> i64 {
        match self {
            OperandType::Byte | OperandType::UByte => 1,
            OperandType::Int | OperandType::UInt => 4,
            OperandType::Long | OperandType::ULong | OperandType::Double => 8,
            OperandType::ByteArray { size } => size,
        }
    }

    /// True when the operand is a 64-bit integer-shaped value (`Long` or
    /// `ULong`).  Used to pick the quadword register-name table in the
    /// emitter and to choose between `cmpl` / `cmpq`.
    pub fn is_long_word(self) -> bool {
        matches!(self, OperandType::Long | OperandType::ULong)
    }

    /// True when the operand is an unsigned integer.
    pub fn is_unsigned(self) -> bool {
        matches!(
            self,
            OperandType::UInt | OperandType::ULong | OperandType::UByte
        )
    }
}

/// Side table that records the type of every TACKY variable in a
/// function.  Populated by the lowerer as it walks the AST and used by
/// the codegen pass to choose between 32-bit and 64-bit operand widths.
/// Variables include the function's parameters, every `VarDecl`-bound
/// local, and every synthetic temporary the lowerer allocates; the
/// chapter-11 surface only needs `Int` and `Long`.
pub type TypeEnv = std::collections::HashMap<String, OperandType>;

/// TACKY value paired with the operand width its assembler form needs.
///
/// The lowerer tags every `Val` it produces (constants and temporaries
/// both) so the codegen pass can pick `addl` vs `addq` without
/// consulting a side table.  Mirrors the OCaml `Tacky.{src;dst}` shape
/// in which every operand is paired with its `asm_type` at construction
/// time (see `nqcc2/lib/tacky_gen.ml:163-167` and
/// `nqcc2/lib/backend/codegen.ml:119-130`).
#[derive(Clone, Debug, PartialEq)]
pub struct TypedVal {
    pub val: Val,
    pub ty: OperandType,
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
#[derive(Clone, Debug, PartialEq)]
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
        scale: i64,
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
/// `params` implicit (the function had no parameters).  Chapter 11
/// adds `type_env` so the codegen pass can look up the operand width
/// of every TACKY variable (parameter / local / synthetic tmp /
/// materialised long constant) without re-walking the AST.
#[derive(Clone, Debug, PartialEq)]
pub struct TackyFunction {
    pub name: String,
    pub global: bool,
    pub params: Vec<String>,
    pub body: Vec<Instruction>,
    pub type_env: TypeEnv,
}

/// A TACKY program: a list of functions.
///
/// Mirrors `nqcc2/lib/tacky.ml` `program`.  The single-function main of
/// chapters 1-7 fits in a `vec![one_entry]`; multi-function support lands in
/// chapters 9+ alongside the chapter-9 function declarations.  Chapter 10
/// widens the surface with `static_variables: Vec<TackyStaticVariable>`
/// (file-scope variable declarations like `int g = 5;`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TackyProgram {
    pub functions: Vec<TackyFunction>,
    pub static_variables: Vec<TackyStaticVariable>,
    pub static_constants: Vec<TackyStaticConstant>,
}

/// Static initializer carried by a file-scope variable declaration.
///
/// Mirrors `nqcc2/lib/tacky.ml` `initial_value` semantics for the
/// chapter-10 surface (`StaticVariable { init : StaticInit }`).  Only
/// integer constants land here today; chapter 11+ adds `Long`, `Double`,
/// zero-fill, and string bytes via the assembly `StaticInit` enum which
/// already has the wider surface declared.
#[derive(Clone, Debug, PartialEq)]
pub struct TackyStaticVariable {
    pub name: String,
    pub init: TackyStaticInit,
    pub global: bool,
    /// Chapter 11: operand width for the static variable.  `Int`
    /// statics are emitted as `.long` (4 bytes) and `Long` statics
    /// as `.quad` (8 bytes).
    pub ty: OperandType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TackyStaticConstant {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TackyStaticInit {
    Int(i64),
    /// Chapter 11: 64-bit static initializer.  Mirrors the
    /// assembly `StaticInit::Long` variant.
    Long(i64),
    /// Chapter 13: 64-bit IEEE-754 double constant for a file-scope
    /// `static double x = 3.14;` initializer.
    Double(f64),
    Char(u8),
    StringBytes(Vec<u8>),
    Pointer(String),
    Aggregate(Vec<TackyStaticInit>),
    /// Placeholder so future chapters can extend the IR without changing
    /// the variant set the lowerer / codegen pre-commit to.
    Zero,
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
