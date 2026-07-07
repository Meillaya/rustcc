// Mirrors nqcc2/lib/backend/codegen.ml.
//
// The OCaml file owns TACKY -> assembly conversion: it walks each TACKY
// function, computes a stack frame, and emits an `AsmProgram`. The real
// implementation lands in wave 2+ once `TackyProgram` and `Frame` have
// their full field sets. Until then, both the program type and the
// generator are unimplemented stubs.
//
// `TackyProgram` is defined inline as a placeholder so this module can
// compile before `src/ir/` grows the full TACKY representation; once that
// representation lands, switch the stub to a type alias for `ir::tacky::Program`.
#![allow(dead_code)]

use anyhow::Result;

use crate::codegen::assembly::AsmProgram;
use crate::codegen::frame::Frame;

/// Placeholder for the TACKY program type. The real `TackyProgram` will be
/// defined in `src/ir/` as part of the wave-2 codegen work and this type will
/// be replaced with a re-export alias.
#[derive(Debug, Default, Clone)]
pub struct TackyProgram {}

/// Generate an assembly program from TACKY input.
///
/// `frames` carries the per-function stack layouts that the codegen pass
/// consults when lowering local accesses and callee arguments. The current
/// stub panics; the chapter-1+ implementation walks each TACKY function,
/// emits prologue/epilogue, and lowers every instruction class.
pub fn generate(_tacky: &TackyProgram, _frames: &[Frame]) -> Result<AsmProgram> {
    unimplemented!("ch.1+ codegen wired in wave 2+")
}