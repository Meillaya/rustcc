//! Semantic-analysis phase.
//!
//! The three-pass facade mirrors `nqcc2/lib/semantic_analysis/`:
//!
//! 1. `resolve` — identifier resolution, scope tracking, label/goto checks,
//!    loop/switch validation.  Mirrors `resolve.ml`.
//! 2. `label_loops` — rewrite `break` / `continue` and `for` / `while` /
//!    `do-while` constructs into explicit jumps targeting generated labels.
//!    Mirrors `label_loops.ml`.
//! 3. `typecheck` — attach types to expressions and enforce C's type
//!    compatibility rules.  Mirrors `typecheck.ml`.
//!
//! The old monolithic `validate.rs` was deleted in wave 6; its scope/label/goto
//! logic will be reintroduced under `resolve` and `label_loops` in later waves.

pub mod label_loops;
pub mod resolve;
pub mod typecheck;

pub use label_loops::label_loops;
pub use resolve::{ResolvedProgram, resolve_program};
pub use typecheck::{TypedProgram, typecheck};
