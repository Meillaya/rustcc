//! External tool invocation boundary.
//!
//! The compiler emits assembly text itself. We still rely on the host C
//! toolchain for preprocessing and for the assembler/linker steps, which is the
//! same split used by the book: implement compiler phases in Rust, delegate
//! object-file and executable production to proven platform tools.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn preprocess(input: &Path, output: &Path) -> Result<()> {
    let status = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(input)
        .arg("-o")
        .arg(output)
        .status()
        .context("failed to launch preprocessor")?;
    if status.success() {
        Ok(())
    } else {
        bail!("preprocessor failed for {}", input.display())
    }
}

pub fn assemble_only(assembly: &Path, object: &Path) -> Result<()> {
    let status = Command::new("gcc")
        .arg("-c")
        .arg(assembly)
        .arg("-o")
        .arg(object)
        .status()
        .context("failed to launch assembler")?;
    if status.success() {
        Ok(())
    } else {
        bail!("assembler failed for {}", assembly.display())
    }
}

pub fn assemble_and_link(assembly: &Path, output: &Path, linker_args: &[String]) -> Result<()> {
    let mut command = Command::new("gcc");
    command.arg(assembly);
    command.args(linker_args);
    command.arg("-o").arg(output);
    let status = command.status().context("failed to launch linker")?;
    if status.success() {
        Ok(())
    } else {
        bail!("assemble/link failed for {}", assembly.display())
    }
}
