//! Lowering placeholders from AST/validated forms into TACKY.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Future lowering façade.
#[derive(Debug, Default)]
pub struct LoweringPass;

impl LoweringPass {
    /// Lower validated frontend output into IR.
    pub fn lower(&self) -> Result<()> {
        bail!("TODO: implement AST-to-IR lowering")
    }
}
