//! rustcc library crate.
//!
//! Exposes the public backend surface so other binaries in `src/bin/`
//! (smoke tests, alternative front-ends) can `use rustcc::codegen::*`.
//! The main binary in `src/main.rs` keeps its own internal `mod`
//! declarations — this library is a parallel compile unit, not the
//! primary crate root.

pub mod codegen;