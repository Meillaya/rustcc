//! Statement AST variants.
//!
//! `Box<Statement>` keeps recursive statement variants finite-sized while still
//! making ownership explicit: each parent owns its child statement subtree.
//!
//! Chapter 8 (loops, break/continue, switch) extends the surface with
//! label/target bookkeeping fields:
//!
//! - `While` / `DoWhile` / `For` carry a `label` set by the
//!   `label_loops` pass.  The lowerer reuses this label to derive the
//!   break-target (`break.<label>`) and continue-target
//!   (`continue.<label>`) assembly labels, matching the OCaml
//!   `nqcc2/lib/tacky_gen.ml` `break_label` / `continue_label` helpers.
//! - `Switch` carries a `label` for the same reason: a bare `break;`
//!   inside a `case` jumps to the switch's break-label, not the
//!   enclosing loop's.
//! - `Break(String)` / `Continue(String)` carry the **target loop's
//!   label** (set by `label_loops`).  The empty string is the parser
//!   placeholder; the semantic pass fills it in or rejects the
//!   statement if no enclosing loop/switch is on the stack.

use super::{
    decl::{BlockItem, ForInit},
    expr::Expr,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Statement {
    Return(Expr),
    Block(Vec<BlockItem>),
    While {
        condition: Expr,
        body: Box<Statement>,
        /// Filled by the `label_loops` pass.  Empty until then.
        label: String,
    },
    DoWhile {
        body: Box<Statement>,
        condition: Expr,
        /// Filled by the `label_loops` pass.  Empty until then.
        label: String,
    },
    For {
        init: Option<ForInit>,
        condition: Option<Expr>,
        post: Option<Expr>,
        body: Box<Statement>,
        /// Filled by the `label_loops` pass.  Empty until then.
        label: String,
    },
    /// `break;` or `break <target>;` (chapter-8 extra).  `target` is
    /// empty until the `label_loops` pass resolves it (for bare
    /// `break;`) or until the parser fills it from the user-written
    /// label.
    Break(String),
    /// `continue;` or `continue <target>;` (chapter-8 extra).
    Continue(String),
    Switch {
        expr: Expr,
        body: Box<Statement>,
        /// Filled by the `label_loops` pass.  Empty until then.
        label: String,
    },
    Case {
        value: Expr,
        statement: Box<Statement>,
    },
    Default {
        statement: Box<Statement>,
    },
    If {
        condition: Expr,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    Goto(String),
    Labeled {
        label: String,
        statement: Box<Statement>,
    },
    Expr(Option<Expr>),
}
