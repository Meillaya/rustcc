//! Future scanner entry points.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Placeholder lexer façade.
#[derive(Debug, Default)]
pub struct Lexer;

impl Lexer {
    /// Construct a lexer once token and cursor plumbing exists.
    pub fn new() -> Self {
        Self
    }

        /// Future hook for the lexer entry point.
        pub fn scan(&self, _source: &str) -> Result<()> {
            bail!("TODO: implement lexical scanning in book order")
        }
}
