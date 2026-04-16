//! IR-to-assembly lowering placeholders.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Future backend lowering façade.
#[derive(Debug, Default)]
pub struct BackendLowering;

impl BackendLowering {
    /// Lower IR into backend assembly forms.
    pub fn lower(&self) -> Result<()> {
        bail!("TODO: implement IR-to-assembly lowering")
    }
}
