//! rustcc library crate (intentionally minimal).
//!
//! The backend types (`AsmProgram`, `Reg`, `Operand`, `Instr`, ...) live
//! in `src/codegen/assembly.rs` and are reachable via the binary's
//! internal `mod codegen;` declaration in `src/main.rs`. The codegen
//! pass that consumes `TackyProgram` lives there too, because
//! `TackyProgram` is a binary-internal type, not a library-public one.
//!
//! Alternative front-ends that want to use the assembly AST and emitter
//! can depend on `rustcc` and `use rustcc::assembly::*` etc.; the codegen
//! driver remains an internal pipeline concern.
