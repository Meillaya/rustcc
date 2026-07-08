// Mirrors nqcc2/lib/backend/codegen.ml ABI section.
//
// The OCaml ABI module classifies each function parameter by where it
// should be passed under the System V AMD64 calling convention: integer
// args in `%rdi/%rsi/%rdx/%rcx/%r8/%r9`, SSE doubles in `%xmm0..%xmm7`,
// and anything beyond the sixth positional arg on the stack. The Rust
// port mirrors the integer-classification half today; the SSE pass for
// `double` lands in chapter 13 and the stack-arg lowering in chapter 18.

/// How a function argument is passed under System V AMD64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamClass {
    /// Passed in one of the six integer registers.
    Int(usize),
    /// Passed in one of the eight SSE registers (chapter 13+).
    Sse(usize),
    /// Passed on the stack.
    Stack,
}

/// Per-call ABI plan consumed by the codegen pass when lowering
/// function arguments and the call / ret sequence.
///
/// `param_classes[i]` mirrors how `params[i]` should be passed:
/// - `ParamClass::Int` → use the `i`-th integer register
///   (`rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`).
/// - `ParamClass::Stack` → push the argument on the stack before
///   the `call` instruction.
#[derive(Debug, Default, Clone)]
pub struct AbiPlan {
    pub param_classes: Vec<ParamClass>,
}

/// The six integer registers used to pass positional arguments under
/// the System V AMD64 calling convention.  Mirrors
/// `int_param_passing_regs` in `nqcc2/lib/backend/codegen.ml:4`.
pub const INT_PARAM_REGS: [Reg; 6] = [Reg::DI, Reg::SI, Reg::DX, Reg::CX, Reg::R8, Reg::R9];
pub const XMM_PARAM_REGS: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

/// Subset of physical registers used by the ABI plan.  Kept as a
/// small enum (rather than reusing `codegen::assembly::Reg`) so the
/// ABI module stays decoupled from the assembly AST and is easy to
/// reason about on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    DI,
    SI,
    DX,
    CX,
    R8,
    R9,
}

/// Classify each parameter by how the System V AMD64 ABI passes it.
/// Chapter 9 supports only integer arguments, so every parameter
/// takes the integer path: the first six land in `%rdi`..`%r9`, the
/// rest are pushed on the stack.
///
/// This mirrors `classify_parameters` in
/// `nqcc2/lib/backend/codegen.ml:266-280`, simplified to the integer
/// case (the OCaml function also walks `Double` and struct types).
pub fn classify_params(types: &[crate::ir::tacky::OperandType]) -> AbiPlan {
    let mut plan = AbiPlan::default();
    let mut int_idx = 0usize;
    let mut sse_idx = 0usize;
    for ty in types {
        if *ty == crate::ir::tacky::OperandType::Double {
            if sse_idx < XMM_PARAM_REGS.len() {
                plan.param_classes.push(ParamClass::Sse(sse_idx));
                sse_idx += 1;
            } else {
                plan.param_classes.push(ParamClass::Stack);
            }
        } else if int_idx < INT_PARAM_REGS.len() {
            plan.param_classes.push(ParamClass::Int(int_idx));
            int_idx += 1;
        } else {
            plan.param_classes.push(ParamClass::Stack);
        }
    }
    plan
}

/// Look up the integer register that holds the `idx`-th positional
/// argument.  Panics if `idx >= 6` (the caller must already have
/// routed `idx >= 6` args to the stack).
pub fn int_param_reg(idx: usize) -> Reg {
    INT_PARAM_REGS[idx]
}
