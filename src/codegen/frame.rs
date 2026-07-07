// Mirrors nqcc2 stack-frame knowledge, scattered across codegen.ml/assembly_symbols.ml.
//
// The real `Frame` will hold per-function stack layout (size, locals, callee
// argument offsets, alignment) once the chapter-7+ backend lands. The
// `FrameLayout` companion stays as the build-side summary used by the
// replace-pseudos pass. Both are unit structs today; future waves will
// flesh them out.
#![allow(dead_code)]

/// Per-function stack frame layout. Populated by the codegen pass once the
/// symbol table, locals, and callee arguments are walked (chapter 7+).
#[derive(Debug, Default, Clone)]
pub struct Frame {}

/// Aggregated layout view used by `replace_pseudos` and `fixup` to translate
/// `Pseudo` / `Stack` operands into concrete `Memory(reg, offset)` accesses.
#[derive(Debug, Default, Clone)]
pub struct FrameLayout {}