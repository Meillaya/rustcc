//! Linear early IR used by the native chapters.
//!
//! This is intentionally much simpler than the book's later TACKY: it is a
//! control-flow envelope for a single function whose expressions are still AST
//! nodes.  An enum is the right fit because the interpreter and lowerer agree on
//! a closed set of instruction shapes, and `match` makes every control-flow case
//! explicit.

use crate::ast::Expr;

#[derive(Debug, Clone)]
pub(crate) enum Instr {
    Declare {
        name: String,
        init: Option<Expr>,
    },
    Expr(Option<Expr>),
    Return(Expr),
    JumpIfZero {
        condition: Expr,
        target: String,
    },
    Switch {
        expr: Expr,
        cases: Vec<(i32, String)>,
        default: Option<String>,
        end: String,
    },
    Jump(String),
    Label(String),
}
