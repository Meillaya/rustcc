// Mirrors nqcc2/lib/emit.ml (Format-based pretty-printing).
//
// The OCaml emitter walks the `AsmProgram` and produces x86-64 AT&T
// assembly text using the `Format` module for indentation. The Rust
// port will produce the equivalent text via `std::fmt::Write`. The real
// implementation lands in wave 2 (chapter 1+).
#![allow(dead_code)]

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;

/// Pretty-print an `AsmProgram` to x86-64 AT&T assembly text.
pub fn emit(_program: &AsmProgram) -> Result<String> {
    unimplemented!("ch.1+ emit wired in wave 2")
}