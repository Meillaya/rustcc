//! Generic control-flow graph construction for TACKY and assembly instructions.
//!
//! Mirrors `nqcc2/lib/cfg.ml:1-341`.  The OCaml source uses a functor over an
//! instruction module; this Rust port uses [`CfgInstruction`] as the small seam
//! that classifies each instruction as a label, terminator, or ordinary op.

#![allow(dead_code)]

mod build;
mod instr;
mod types;

pub use build::{
    CfgBuildError, assembly_function_cfg, build, build_tacky_program, tacky_function_cfg,
};
pub use instr::{CfgInstruction, SimpleInstr};
pub use types::{AssemblyCfg, BasicBlock, BlockId, Cfg, FunctionCfg, NodeId, TackyCfg};
