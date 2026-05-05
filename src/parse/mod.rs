//! Parse phase.
//!
//! `parse_program` is the only crate-internal entry point used by the compiler
//! facade.  Cursor and precedence modules remain reserved for future tightening,
//! while the current recursive-descent implementation lives in `parser`.

pub(crate) mod cursor;
pub(crate) mod parser;
pub(crate) mod precedence;

pub(crate) use parser::parse_program;
