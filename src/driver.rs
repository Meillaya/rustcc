//! Compile-driver orchestration.
//!
//! The driver owns CLI parsing, artifact policy, and handoff between the Rust
//! compiler core and the external toolchain. Keeping these responsibilities here
//! makes later compiler phases easier to test: `compiler.rs` can focus on source
//! semantics while this module enforces the official harness contract.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::compiler::{self, CompileOptions};
use crate::toolchain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Lex,
    Parse,
    Validate,
    Tacky,
    Codegen,
    Run,
}

impl Stage {
    fn is_stdout_only(self) -> bool {
        !matches!(self, Self::Run)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactMode {
    StdoutOnly,
    AssemblyFile,
    ObjectFile,
    Executable,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationFlags {
    pub fold_constants: bool,
    pub eliminate_unreachable_code: bool,
    pub propagate_copies: bool,
    pub eliminate_dead_stores: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegallocOptions {
    pub coalescing_enabled: bool,
}

impl Default for RegallocOptions {
    fn default() -> Self {
        Self {
            coalescing_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub input: PathBuf,
    pub stage: Stage,
    pub artifact_mode: ArtifactMode,
    pub optimization_flags: OptimizationFlags,
    pub regalloc_options: RegallocOptions,
    pub linker_args: Vec<String>,
}

impl Config {
    pub fn from_args<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
    {
        let mut input = None;
        let mut stage = Stage::Run;
        let mut explicit_artifact_mode = None;
        let mut optimization_flags = OptimizationFlags::default();
        let mut regalloc_options = RegallocOptions::default();
        let mut linker_args = Vec::new();

        for arg in args.into_iter().skip(1) {
            match arg.as_str() {
                "--lex" | "-l" => stage = Stage::Lex,
                "--parse" | "-p" => stage = Stage::Parse,
                "--validate" => stage = Stage::Validate,
                "--tacky" => stage = Stage::Tacky,
                "--codegen" | "-cg" => stage = Stage::Codegen,
                "--all" | "--run" => stage = Stage::Run,
                "-S" => explicit_artifact_mode = Some(ArtifactMode::AssemblyFile),
                "-c" => explicit_artifact_mode = Some(ArtifactMode::ObjectFile),
                "--fold-constants" => optimization_flags.fold_constants = true,
                "--eliminate-unreachable-code" => {
                    optimization_flags.eliminate_unreachable_code = true;
                }
                "--propagate-copies" => optimization_flags.propagate_copies = true,
                "--eliminate-dead-stores" => optimization_flags.eliminate_dead_stores = true,
                "--no-coalescing" => regalloc_options.coalescing_enabled = false,
                "-lm" => linker_args.push(arg),
                "-h" | "--help" => bail!(help_text()),
                _ if arg.starts_with('-') => bail!("unknown flag: {arg}"),
                _ if input.is_none() => input = Some(PathBuf::from(arg)),
                _ => bail!("unexpected extra positional argument: {arg}"),
            }
        }

        let input =
            input.context("missing input file\n\nusage: rustcc [stage/options] <input.c>")?;
        let artifact_mode = if stage.is_stdout_only() {
            ArtifactMode::StdoutOnly
        } else {
            explicit_artifact_mode.unwrap_or(ArtifactMode::Executable)
        };
        Ok(Self {
            input,
            stage,
            artifact_mode,
            optimization_flags,
            regalloc_options,
            linker_args,
        })
    }
}

pub fn run_from_env() -> Result<()> {
    run(Config::from_args(env::args())?)
}

pub fn run(config: Config) -> Result<()> {
    validate_input_path(&config.input)?;
    let paths = DerivedPaths::from_input(&config.input)?;
    cleanup_artifacts(&paths)?;
    let result = run_checked(config, &paths);
    if result.is_err() {
        cleanup_artifacts(&paths)?;
    }
    result
}

fn run_checked(config: Config, paths: &DerivedPaths) -> Result<()> {
    toolchain::preprocess(&config.input, &paths.preprocessed)?;
    let source = fs::read_to_string(&paths.preprocessed)
        .with_context(|| format!("failed to read {}", paths.preprocessed.display()))?;
    remove_if_exists(&paths.preprocessed)?;

    let artifacts = compiler::compile(
        &source,
        CompileOptions::new(
            config.stage,
            config.optimization_flags,
            config.regalloc_options,
        )
        .with_source_path_hint(config.input.to_string_lossy().into_owned()),
    )?;

    if config.stage.is_stdout_only() {
        let text = match config.stage {
            Stage::Lex => artifacts.tokens_pretty,
            Stage::Parse => artifacts.ast_pretty,
            Stage::Validate => artifacts.typed_ast_pretty,
            Stage::Tacky => artifacts.tacky_pretty,
            Stage::Codegen => artifacts.assembly_text,
            Stage::Run => unreachable!("run is not stdout-only"),
        }
        .context("compiler did not produce requested stage output")?;
        println!("{text}");
        return Ok(());
    }

    let assembly_text = artifacts
        .assembly_text
        .context("compiler did not produce assembly")?;
    match config.artifact_mode {
        ArtifactMode::StdoutOnly => unreachable!("handled above"),
        ArtifactMode::AssemblyFile => {
            fs::write(&paths.assembly, assembly_text)
                .with_context(|| format!("failed to write {}", paths.assembly.display()))?;
        }
        ArtifactMode::ObjectFile => {
            fs::write(&paths.assembly, assembly_text)
                .with_context(|| format!("failed to write {}", paths.assembly.display()))?;
            toolchain::assemble_only(&paths.assembly, &paths.object)?;
            remove_if_exists(&paths.assembly)?;
        }
        ArtifactMode::Executable => {
            fs::write(&paths.assembly, assembly_text)
                .with_context(|| format!("failed to write {}", paths.assembly.display()))?;
            toolchain::assemble_and_link(&paths.assembly, &paths.output, &config.linker_args)?;
            remove_if_exists(&paths.assembly)?;
        }
    }
    Ok(())
}

fn validate_input_path(input: &Path) -> Result<()> {
    if !input.exists() {
        bail!("input file does not exist: {}", input.display());
    }
    match input.extension().and_then(|ext| ext.to_str()) {
        Some("c") => Ok(()),
        _ => bail!("input must be a .c file: {}", input.display()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedPaths {
    pub preprocessed: PathBuf,
    pub assembly: PathBuf,
    pub object: PathBuf,
    pub output: PathBuf,
}

impl DerivedPaths {
    pub fn from_input(input: &Path) -> Result<Self> {
        let stem = input
            .file_stem()
            .context("input path is missing a valid file stem")?;
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        let stem = stem.to_string_lossy();
        Ok(Self {
            preprocessed: parent.join(format!("{stem}.i")),
            assembly: parent.join(format!("{stem}.s")),
            object: parent.join(format!("{stem}.o")),
            output: parent.join(stem.as_ref()),
        })
    }

    pub fn cleanup_targets(&self) -> [&Path; 3] {
        [
            self.assembly.as_path(),
            self.object.as_path(),
            self.output.as_path(),
        ]
    }
}

fn cleanup_artifacts(paths: &DerivedPaths) -> Result<()> {
    for target in paths.cleanup_targets() {
        remove_if_exists(target)?;
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove {}", path.display())),
    }
}

fn help_text() -> &'static str {
    "usage: rustcc [--lex|--parse|--validate|--tacky|--codegen|-S|-c] [options] <input.c>"
}

#[cfg(test)]
mod tests {
    use super::{ArtifactMode, Config, DerivedPaths, OptimizationFlags, RegallocOptions, Stage};
    use std::path::PathBuf;

    #[test]
    fn parses_default_run_stage() {
        let config = Config::from_args(["rustcc".into(), "demo.c".into()]).unwrap();
        assert_eq!(config.stage, Stage::Run);
        assert_eq!(config.artifact_mode, ArtifactMode::Executable);
    }

    #[test]
    fn parses_stage_flags_as_stdout_only() {
        for (flag, stage) in [
            ("--lex", Stage::Lex),
            ("--parse", Stage::Parse),
            ("--validate", Stage::Validate),
            ("--tacky", Stage::Tacky),
            ("--codegen", Stage::Codegen),
        ] {
            let config =
                Config::from_args(["rustcc".into(), flag.into(), "demo.c".into()]).unwrap();
            assert_eq!(config.stage, stage);
            assert_eq!(config.artifact_mode, ArtifactMode::StdoutOnly);
        }
    }

    #[test]
    fn parses_artifact_and_feature_flags() {
        let config = Config::from_args([
            "rustcc".into(),
            "-c".into(),
            "--fold-constants".into(),
            "--eliminate-unreachable-code".into(),
            "--propagate-copies".into(),
            "--eliminate-dead-stores".into(),
            "--no-coalescing".into(),
            "-lm".into(),
            "demo.c".into(),
        ])
        .unwrap();
        assert_eq!(config.artifact_mode, ArtifactMode::ObjectFile);
        assert_eq!(
            config.optimization_flags,
            OptimizationFlags {
                fold_constants: true,
                eliminate_unreachable_code: true,
                propagate_copies: true,
                eliminate_dead_stores: true,
            }
        );
        assert_eq!(
            config.regalloc_options,
            RegallocOptions {
                coalescing_enabled: false,
            }
        );
        assert_eq!(config.linker_args, vec!["-lm"]);
    }

    #[test]
    fn derives_all_output_paths() {
        let paths = DerivedPaths::from_input(&PathBuf::from("examples/demo.c")).unwrap();
        assert_eq!(paths.preprocessed, PathBuf::from("examples/demo.i"));
        assert_eq!(paths.assembly, PathBuf::from("examples/demo.s"));
        assert_eq!(paths.object, PathBuf::from("examples/demo.o"));
        assert_eq!(paths.output, PathBuf::from("examples/demo"));
    }
}
