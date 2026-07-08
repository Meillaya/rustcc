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

/// Unary operators supported through chapter 4.
///
/// `Negate` and `Complement` come from chapter 2 (book Listing 2-2).
/// `Not` is the logical-not operator introduced in chapter 4
/// (`!e`).  It is distinct from `Complement` (`~e`, bitwise NOT):
/// `!0 == 1`, but `~0 == -1`.  The two operators must be kept
/// apart at every pass that consumes them (parser, lowering,
/// codegen) because their semantics differ — `Complement` is a
/// bitwise flip and lowers to a `notl`, while `Not` is a
/// boolean normalization and lowers to `cmpl $0, src; sete dst`
/// (followed by a `movzbl` to widen the byte-sized result).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    /// `-<expr>` — additive inverse.
    Negate,
    /// `~<expr>` — bitwise NOT.
    Complement,
    /// `!<expr>` — logical NOT (chapter 4).
    Not,
}

/// Binary operators supported through chapter 4.
///
/// Chapter 3 contributes the arithmetic / modulo operators; the
/// extra-credit chapter 3 cases contribute the bitwise operators
/// (`&`, `|`, `^`, `<<`, `>>`); chapter 4 contributes the
/// equality (`==`, `!=`), relational (`<`, `<=`, `>`, `>=`),
/// and logical (`&&`, `||`) operators.  `&&` and `||` are
/// lowered with short-circuit semantics; the equality and
/// relational operators are lowered to a TACKY `Cmp`
/// instruction that is then normalized to 0/1 at codegen time.
///
/// Mirrors `nqcc2/lib/parse.ml` `parse_binop` (chapter 4 listing).
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
    // Chapter 4 — equality / relational / logical.
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    LogicalAnd,
    LogicalOr,
}
