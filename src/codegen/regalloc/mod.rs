// Mirrors nqcc2/lib/backend/regalloc.ml (651 LOC; uses Briggs/George coalescing).
//
// Chapter 20 starts with the liveness foundation used by later interference
// graph construction. Coloring, spilling, and coalescing remain intentionally
// out of scope for W21-T1.
#![allow(dead_code)]

use anyhow::Result;
use std::error::Error;
use std::fmt;

use crate::codegen::assembly::{AsmProgram, Instr};
use crate::ir::cfg::{self, CfgBuildError};

mod liveness;
mod operands;
mod types;

pub use liveness::{LiveBlock, LiveCfg, analyze as analyze_liveness, block_liveness};
pub use operands::{UseDef, regs_used_and_written};
pub use types::{BlockLiveness, LiveMap, LiveSet, LivenessConfig, LivenessError, RegisterClass};

#[derive(Debug)]
pub enum LivenessAnalysisError {
    Cfg(CfgBuildError),
    Liveness(LivenessError),
}

impl fmt::Display for LivenessAnalysisError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cfg(err) => write!(out, "{err}"),
            Self::Liveness(err) => write!(out, "{err}"),
        }
    }
}

impl Error for LivenessAnalysisError {}

impl From<CfgBuildError> for LivenessAnalysisError {
    fn from(err: CfgBuildError) -> Self {
        Self::Cfg(err)
    }
}

impl From<LivenessError> for LivenessAnalysisError {
    fn from(err: LivenessError) -> Self {
        Self::Liveness(err)
    }
}

pub fn analyze_function_liveness(
    fn_name: &str,
    instructions: &[Instr],
    class: RegisterClass,
    config: &LivenessConfig,
) -> std::result::Result<LiveCfg, LivenessAnalysisError> {
    let cfg = cfg::assembly_function_cfg(fn_name, instructions)?;
    analyze_liveness(cfg, class, config).map_err(Into::into)
}

/// Assign a physical register to every `Reg` use in the assembly, spilling
/// when the available callee-saved set is exhausted.
pub fn allocate(_asm: AsmProgram) -> Result<AsmProgram> {
    unimplemented!("ch.20 regalloc wired in wave 21")
}
