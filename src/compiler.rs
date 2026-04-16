//! Public boundary for compiler internals.
//!
//! The driver should call into this module using preprocessed source text
//! rather than touching lexer/parser/codegen details directly.

use anyhow::{Result, bail};

use crate::driver::Stage;

/// Placeholder compile entry point.
#[allow(dead_code)]
pub fn compile(_source: &str, _stage: Stage) -> Result<()> {
    bail!("TODO: implement compiler boundary for lex/parse/codegen/full stages");
}
