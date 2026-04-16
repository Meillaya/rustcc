//! Semantic validation entry points.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Future semantic-validation façade.
#[derive(Debug, Default)]
pub struct Validator;

impl Validator {
    /// Validate a parsed translation unit once AST and symbol surfaces exist.
    pub fn validate(&self) -> Result<()> {
        bail!("TODO: implement semantic validation in book order")
    }
}
