// Mirrors nqcc2/lib/backend/regalloc.ml (651 LOC; uses Briggs/George coalescing).
//
// Chapter 20 builds register allocation incrementally: liveness, interference,
// conservative coalescing, simplification, select/color, and bounded spill
// reallocation.
#![allow(dead_code)]

use anyhow::Result;
use std::error::Error;
use std::fmt;

use std::collections::HashSet;

use crate::codegen::assembly::{AsmProgram, Instr};
use crate::driver::RegallocOptions;
use crate::ir::cfg::{self, CfgBuildError};

mod abi_liveness;
mod allocate;
mod coalesce;
mod color;
mod division_copy;
mod graph;
mod graph_pseudos;
mod liveness;
mod operands;
mod rewrite;
mod scratch;
mod simplify;
mod spill;
mod types;

pub use color::{ColorMap, SelectResult, select};
pub use graph::{
    InterferenceBuild, InterferenceConfig, InterferenceGraph, InterferenceNode,
    NodeId as InterferenceNodeId, NodeSet as InterferenceNodeSet, build_interference, hardreg_node,
    is_hardreg_node,
};
pub use liveness::{LiveBlock, LiveCfg, analyze as analyze_liveness, block_liveness};
pub use operands::{UseDef, instr_operands, regs_used_and_written};
pub use simplify::{Simplification, SimplifyChoice, SimplifyStep, simplify};
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

/// Assign physical registers where coloring succeeds; uncolored pseudos stay
/// as spill markers for `replace_pseudos` to place on the stack.
pub fn allocate(
    asm: AsmProgram,
    globals: &HashSet<String>,
    options: RegallocOptions,
) -> Result<AsmProgram> {
    allocate::allocate(asm, globals, options)
}
