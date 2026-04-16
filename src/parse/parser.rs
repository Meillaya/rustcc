//! Parser façade and top-level parsing entry points.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Placeholder parser façade.
#[derive(Debug, Default)]
pub struct Parser;

impl Parser {
    /// Construct a parser once the token cursor exists.
    pub fn new() -> Self {
        Self
    }

        /// Future hook for the parser entry point.
        pub fn translation_unit_placeholder(&self) -> Result<()> {
            bail!("TODO: implement parsing in book order")
        }
}
