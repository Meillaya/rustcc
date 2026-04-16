//! External tool invocation boundary.
//!
//! Keep subprocess details here so the driver can stay focused on pipeline
//! orchestration.

use std::path::Path;

use anyhow::{Result, bail};

/// Placeholder for the preprocessing step.
#[allow(dead_code)]
pub fn preprocess(_input: &Path, _output: &Path) -> Result<()> {
    bail!("TODO: implement preprocessing with the system C toolchain");
}

/// Placeholder for the final assemble/link step.
#[allow(dead_code)]
pub fn assemble_and_link(_assembly: &Path, _output: &Path) -> Result<()> {
    bail!("TODO: implement assemble/link with the system C toolchain");
}
