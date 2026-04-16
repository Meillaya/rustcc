//! Compile-driver orchestration scaffold.
//!
//! This module is intentionally small and incomplete: it defines the public
//! control-flow surface for the compile driver without implementing the real
//! pipeline yet.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// The compilation stage at which the driver should stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Preprocess,
    Lex,
    Parse,
    Codegen,
    Full,
}

/// User intent collected from the command line.
#[derive(Debug, Clone)]
pub struct Config {
    pub input: PathBuf,
    pub stage: Stage,
}

impl Config {
    /// Parse the current process arguments into a driver configuration.
    ///
    /// This is a scaffold parser: it accepts a single input path plus one
    /// optional stop-stage flag.
    pub fn from_args<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
    {
        let mut input: Option<PathBuf> = None;
        let mut stage = Stage::Full;

        for arg in args.into_iter().skip(1) {
            match arg.as_str() {
                "--lex" | "-l" => stage = Stage::Lex,
                "--parse" | "-p" => stage = Stage::Parse,
                "--codegen" | "-cg" => stage = Stage::Codegen,
                "--preprocess" => stage = Stage::Preprocess,
                "--all" => stage = Stage::Full,
                "-h" | "--help" => {
                    bail!(help_text());
                }
                _ if arg.starts_with('-') => {
                    bail!("unknown flag: {arg}");
                }
                _ if input.is_none() => input = Some(PathBuf::from(arg)),
                _ => bail!("unexpected extra positional argument: {arg}"),
            }
        }

        let input = input.context("missing input file\n\nusage: rustcc [--lex|--parse|--codegen|--preprocess|--all] <input.c>")?;

        Ok(Self { input, stage })
    }
}

/// Entry point used by `main.rs`.
pub fn run_from_env() -> Result<()> {
    let config = Config::from_args(env::args())?;
    run(config)
}

/// The compile-driver pipeline.
///
/// Current status:
/// - validates the input path shape
/// - computes derived paths
/// - deliberately stops before doing real toolchain/compiler work
pub fn run(config: Config) -> Result<()> {
    validate_input_path(&config.input)?;

    let paths = DerivedPaths::from_input(&config.input)?;

    println!("compile driver scaffold ready");
    println!("input: {}", config.input.display());
    println!("stage: {:?}", config.stage);
    println!("preprocessed: {}", paths.preprocessed.display());
    println!("assembly: {}", paths.assembly.display());
    println!("output: {}", paths.output.display());
    println!();
    println!("next implementation steps:");
    println!("1. run preprocessor via toolchain.rs");
    println!("2. stop early for preprocess-only mode");
    println!("3. read the .i file");
    println!("4. call the compiler boundary in compiler.rs");
    println!("5. assemble/link for full builds");

    Ok(())
}

fn validate_input_path(input: &PathBuf) -> Result<()> {
    if !input.exists() {
        bail!("input file does not exist: {}", input.display());
    }

    match input.extension().and_then(|ext| ext.to_str()) {
        Some("c") => Ok(()),
        _ => bail!("input must be a .c file: {}", input.display()),
    }
}

#[derive(Debug, Clone)]
struct DerivedPaths {
    preprocessed: PathBuf,
    assembly: PathBuf,
    output: PathBuf,
}

impl DerivedPaths {
    fn from_input(input: &PathBuf) -> Result<Self> {
        let stem = input
            .file_stem()
            .context("input path is missing a valid file stem")?;
        let parent = input.parent().unwrap_or_else(|| std::path::Path::new("."));

        Ok(Self {
            preprocessed: parent.join(format!("{}.i", stem.to_string_lossy())),
            assembly: parent.join(format!("{}.s", stem.to_string_lossy())),
            output: parent.join(stem),
        })
    }
}

fn help_text() -> &'static str {
    "usage: rustcc [--lex|--parse|--codegen|--preprocess|--all] <input.c>"
}

#[cfg(test)]
mod tests {
    use super::{Config, DerivedPaths, Stage};
    use std::path::PathBuf;

    #[test]
    fn parses_default_full_stage() {
        let config = Config::from_args(["rustcc".into(), "demo.c".into()]).unwrap();

        assert_eq!(config.stage, Stage::Full);
        assert_eq!(config.input, PathBuf::from("demo.c"));
    }

    #[test]
    fn parses_explicit_stage_flag() {
        let config = Config::from_args(["rustcc".into(), "--parse".into(), "demo.c".into()]).unwrap();

        assert_eq!(config.stage, Stage::Parse);
        assert_eq!(config.input, PathBuf::from("demo.c"));
    }

    #[test]
    fn derives_intermediate_paths_from_input_stem() {
        let paths = DerivedPaths::from_input(&PathBuf::from("examples/demo.c")).unwrap();

        assert_eq!(paths.preprocessed, PathBuf::from("examples/demo.i"));
        assert_eq!(paths.assembly, PathBuf::from("examples/demo.s"));
        assert_eq!(paths.output, PathBuf::from("examples/demo"));
    }
}
