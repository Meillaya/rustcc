//! Backend and assembly text utilities.
//!
//! `lower` contains the small native constant-return emitter used for early
//! chapters.  `emit` contains bridge assembly sanitation for advanced chapters
//! that still rely on the host C backend.  ABI/frame/register-allocation files
//! remain named placeholders for future native backend ownership.

pub(crate) mod abi;
pub(crate) mod asm;
pub(crate) mod emit;
pub(crate) mod frame;
pub(crate) mod lower;
pub(crate) mod register_allocator;

pub(crate) use emit::{SystemAssemblySanitizerOptions, sanitize_system_assembly};
pub(crate) use lower::emit_native_constant_function;
