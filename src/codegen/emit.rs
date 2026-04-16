//! Assembly-emission placeholders.

#![allow(dead_code)]

use anyhow::{bail, Result};

/// Future assembly emitter façade.
#[derive(Debug, Default)]
pub struct Emitter;

impl Emitter {
    /// Emit final assembly text from backend data structures.
    pub fn emit(&self) -> Result<String> {
        bail!("TODO: implement assembly emission")
    }
}
