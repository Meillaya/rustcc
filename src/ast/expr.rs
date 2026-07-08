//! Expression AST variants.
//!
//! Expressions form a recursive tree, so child expressions are boxed. `enum`
//! plus pattern matching makes every compiler phase handle each expression kind
//! explicitly.

use super::operator::{AssignOp, BinaryOp, UnaryOp};
use super::ty::Type;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr {
    Constant(i64),
    /// Chapter 11: integer constant with an `L` / `l` suffix in the
    /// source.  Lowered as a typed `long` regardless of the value's
    /// magnitude (e.g. `0l` is `long`, not `int`).  Carries the
    /// original `i64` so the constant can be reused as-is in the IR
    /// without truncation.
    LongConstant(i64),
    /// Chapter 12: unsigned constant `123u`, `123ul`, etc.  The
    /// companion `bool` is `true` for `unsigned long` (the `U` /
    /// `uL` / `lU` / `LU` suffix cases), `false` for plain
    /// `unsigned int` (`U` / `u`).
    UIntConstant(i64, bool),
    /// Chapter 13: floating-point constant `3.14`, `1e-5`, etc.
    DoubleConstant(f64),
    StringLiteral(String),
    Var(String),
    /// Chapter 11: explicit cast `(T) expr`.  The lowerer turns
    /// this into `SignExtend` (int -> long) or `Truncate`
    /// (long -> int).  Mirrors `Tacky.Cast` in the OCaml reference.
    Cast {
        target_type: Type,
        expr: Box<Expr>,
    },
    SizeOfExpr(Box<Expr>),
    SizeOfType(Type),
    Paren(Box<Expr>),
    /// A unary operation. Carries the operator kind via [`UnaryOp`] so the
    /// parser, lowerer, and codegen can dispatch on a single field.  Covers
    /// `-` (`Negate`), `~` (`Complement`), and chapter-4 `!` (`Not`).
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    PreInc(Box<Expr>),
    PreDec(Box<Expr>),
    PostInc(Box<Expr>),
    PostDec(Box<Expr>),
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    /// A binary operation. Carries the operator kind via [`BinaryOp`] so the
    /// parser, lowerer, and codegen can dispatch on a single field.  Covers
    /// chapter-3 arithmetic / bitwise / shift operators and the chapter-4
    /// equality / relational / logical operators.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Chapter 9: a function call `name(args)`.  Arguments are full
    /// expressions (so any chapter-3/4/5 expression shape can be passed);
    /// the call itself yields an `int` value that can be used in any
    /// expression context (assignment, return, arithmetic, etc.).
    Call {
        name: String,
        args: Vec<Expr>,
    },
    /// Chapter 14: address-of `&lvalue`.  The lowerer emits a
    /// `GetAddress` for the inner lvalue and tags the result as a
    /// pointer.  Mirrors `Ast.AddrOf` in the OCaml reference.
    AddressOf(Box<Expr>),
    /// Chapter 14: dereference `*pointer`.  In an rvalue context the
    /// lowerer emits a `Load`; in an lvalue context (e.g. `*p = 5;`)
    /// it becomes a `Store` target.  Mirrors `Ast.Dereference`.
    Dereference(Box<Expr>),
    /// Chapter 15: subscript `base[index]`.  Semantically
    /// `*(base + index)`; the lowerer emits an `AddPtr` followed by
    /// a `Load`/`Store`.  Mirrors `Ast.Subscript`.
    Subscript {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    InitializerList(Vec<Expr>),
}

impl Expr {
    /// Return the variable name when this expression can be assigned to.
    /// Parentheses preserve lvalue-ness for cases like `++(a)`, but assignment
    /// results do not become lvalues, matching the invalid increment tests.
    /// Chapter 14+ lvalues (`*p`, `arr[i]`) need a `Load`/`Store` instead of
    /// a `Copy`, so the lowerer special-cases them outside this helper.
    pub(crate) fn lvalue_name(&self) -> Option<&str> {
        match self {
            Self::Var(name) => Some(name),
            Self::Paren(inner) => inner.lvalue_name(),
            _ => None,
        }
    }

    /// True when the expression designates a memory location that can be
    /// assigned to or have its address taken.  Mirrors the OCaml
    /// `is_lvalue` helper.
    pub(crate) fn is_lvalue(&self) -> bool {
        match self {
            Self::Var(_) => true,
            Self::Paren(inner) => inner.is_lvalue(),
            Self::Dereference(_) | Self::Subscript { .. } | Self::StringLiteral(_) => true,
            _ => false,
        }
    }
}
