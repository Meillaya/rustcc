// Mirrors nqcc2/lib/backend/assembly_symbols.ml.
//
// The OCaml module aggregates the global / extern symbol sets that the
// assembly emitter needs: which symbols are defined in this translation
// unit, which are imported, which are constants (chapter 10+), and which
// top-level items are static. The Rust port keeps the surface as a single
// struct today and will grow fields as the codegen waves add static
// variables, constants, and extern declarations.
#![allow(dead_code)]

/// Aggregated symbol table for the assembly emitter.
#[derive(Debug, Default, Clone)]
pub struct AsmSymbols {}