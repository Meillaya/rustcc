//! Semantic-analysis phase.
//!
//! The facade calls `validate_program` after parsing.  The implementation lives
//! in `validate`, while `names`, `symbols`, `types`, and `layout` remain stable
//! homes for future native type and ABI work as later bridge-backed chapters are
//! replaced.

pub(crate) mod layout;
pub(crate) mod names;
pub(crate) mod symbols;
pub(crate) mod types;
pub(crate) mod validate;

pub(crate) use validate::validate_program;
