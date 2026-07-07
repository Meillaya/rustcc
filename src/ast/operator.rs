//! Operators used by expression AST nodes.
//!
//! Operators are small `Copy` enums because they are closed sets of symbolic
//! choices and have no owned data. This keeps parser/evaluator code simple and
//! avoids unnecessary cloning of expression trees.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssignOp {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    /// `-<expr>` — additive inverse.
    Negate,
    /// `~<expr>` — bitwise NOT.
    Complement,
}

/// Binary operators supported through chapter 3 plus the bitwise extras.
///
/// Mirrors the operators `nqcc2/lib/parse.ml` covers in `parse_binop`
/// (Listing 3-1) for chapter 3, plus the bitwise extras covered in the
/// extra-credit chapter 3 cases (`&`, `|`, `^`, `<<`, `>>`).  The
/// relational, equality, and logical operators arrive in chapter 4 and
/// are intentionally absent here — chapter-4 programs should fail at
/// parse time because the chapter-3 grammar does not accept them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
}