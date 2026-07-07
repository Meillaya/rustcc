// Mirrors nqcc2/lib/backend/codegen.ml ABI section.
//
// The OCaml ABI module classifies each function parameter by where it
// should be passed under the System V AMD64 calling convention: integer
// args in `%rdi/%rsi/%rdx/%rcx/%r8/%r9`, SSE doubles in `%xmm0..%xmm7`,
// and anything beyond the eighth positional arg on the stack. The Rust
// port keeps the same classification today; future waves will grow
// `AbiPlan` with the per-call argument layout consumed by the codegen
// pass.
#![allow(dead_code)]

/// How a function argument is passed under System V AMD64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamClass {
    /// Passed in one of the six integer registers.
    Int,
    /// Passed in one of the eight SSE registers.
    SSE,
    /// Passed on the stack (8th positional arg or later).
    Memory,
}

/// Per-call ABI plan consumed by the codegen pass when lowering
/// function arguments and the call / ret sequence.
#[derive(Debug, Default, Clone)]
pub struct AbiPlan {}