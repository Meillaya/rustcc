//! Backend and assembly text utilities.
//!
//! The codegen module mirrors `nqcc2/lib/backend/` plus `nqcc2/lib/emit.ml`:
//!
//! - [`assembly`] is the x86-64 AST introduced by W0-T4 (the type system
//!   only — no printing or codegen logic).
//! - [`assembly_symbols`] tracks the global / extern / constant symbol
//!   sets the emitter needs.
//! - [`abi`] classifies function parameters for the System V AMD64
//!   calling convention.
//! - [`frame`] defines the per-function stack-frame layout.
//! - [`codegen`] walks TACKY and produces an `AsmProgram` (wave 2+).
//! - [`fixup`] rewrites two-operand assembly into explicit move + op
//!   pairs and adds the `cqo` / `cdq` setup division needs (wave 10).
//! - [`replace_pseudos`] resolves `Pseudo` operands into physical
//!   registers or `Stack` slots (wave 21).
//! - [`regalloc`] assigns physical registers via Briggs/George coloring
//!   (wave 21).
//! - [`emit`] pretty-prints an `AsmProgram` to x86-64 AT&T text (wave 2).
//!
//! All of the pass entry points (`generate`, `fixup`, `replace_pseudos`,
//! `allocate`, `emit`) are intentionally unimplemented stubs today; the
//! real implementations land in future waves.

// Re-exports below are the scaffolded public API surface; silence the
// "unused import" diagnostic until downstream wiring lands.
#![allow(unused_imports)]

pub mod abi;
pub mod assembly;
pub mod assembly_symbols;
pub mod codegen;
pub mod emit;
pub mod fixup;
pub mod frame;
pub mod regalloc;
pub mod replace_pseudos;
pub mod type_table;

pub use assembly::{
    AsmProgram, BinaryOpInstr, ConditionCode, Instr, Operand, Reg, StaticInit, TopLevel,
    UnaryOpInstr,
};
pub use codegen::generate;
pub use emit::emit;
pub use fixup::fixup;
pub use regalloc::allocate;
pub use replace_pseudos::replace_pseudos;
