//! Register-allocation placeholders for the final backend chapters.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Future register-allocation façade.
#[derive(Debug, Default)]
pub struct RegisterAllocator;

impl RegisterAllocator {
    /// Allocate physical registers or spills for backend temporaries.
    pub fn allocate(&self) -> Result<()> {
        bail!("TODO: implement register allocation")
    }
}
