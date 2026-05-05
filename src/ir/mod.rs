//! Early native intermediate representation.
//!
//! The current native path lowers the parsed/validated AST into a small linear
//! control-flow envelope and evaluates it to a constant return value.  Later
//! chapters can replace this with full TACKY while preserving the facade entry
//! point used today.

pub(crate) mod control_flow;
pub(crate) mod lower;
pub(crate) mod opt;
pub(crate) mod tacky;

pub(crate) use control_flow::evaluate_program;
