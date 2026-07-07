//! Control-flow graph for TACKY functions.
//!
//! Mirrors `nqcc2/lib/optimizations/cfg.ml` (341 LOC; functor over instruction
//! type).  The OCaml module is parameterised over the instruction type so the
//! same `Cfg` shape is shared between TACKY (`Instruction`) and assembly
//! (`AsmInstr`).  In Rust the same effect is achieved with a generic `Cfg<N>`.
//!
//! Wave 20 will wire the real functor; this stub keeps the surface area small
//! and panics if anything tries to build a CFG before then.

#![allow(dead_code)]

use crate::ir::tacky::{Instruction, TackyProgram};

/// A control-flow graph: a list of named basic blocks plus `entry`/`exit`
/// labels.  Generic over the node payload so the same struct carries TACKY
/// instructions or assembly instructions.
#[derive(Clone, Debug)]
pub struct Cfg<N> {
    pub blocks: Vec<(String, Vec<N>)>,
    pub entry: String,
    pub exit: String,
}

/// Build a CFG from a TACKY program.
///
/// The real implementation constructs one CFG per function, splits the
/// instruction list into basic blocks at labels and jumps, and threads
/// successor edges into the `blocks` list.  The placeholder exists so the
/// optimization passes can be sketched before wave 20 lands the functor.
pub fn build(_program: &TackyProgram) -> Cfg<Instruction> {
    unimplemented!("ch.19 CFG functor wired in W20-T1");
}
