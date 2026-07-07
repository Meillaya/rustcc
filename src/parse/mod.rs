//! Parse phase.
//!
//! `parse_program` is the only crate-internal entry point used by the compiler
//! facade. The recursive-descent implementation lives in `parser`; the
//! `precedence` module is reserved for a future explicit precedence-climbing
//! expression strategy once binary and logical operators arrive.

pub(crate) mod parser;
pub(crate) mod precedence;

pub(crate) use parser::parse_program;
