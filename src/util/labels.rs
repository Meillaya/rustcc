//! Label generator for short-circuit and structured-control-flow lowering.
//!
//! Mirrors the role of `nqcc2/lib/util/unique_ids.ml::make_label`.  The OCaml
//! reference uses a single monotonic counter shared between temporaries and
//! labels; the Rust port keeps them separate so the two name spaces stay
//! syntactically distinct (`tmp.N` vs `prefix.M`).  Callers pass a human
//! prefix (e.g. `"and_false"`, `"or_true"`, `"end"`) and the generator
//! returns a unique name of the form `prefix.N`.

#![allow(dead_code)]

/// Monotonic generator for fresh labels.
///
/// Construct one per lowering (so each pass gets a fresh, monotonically
/// increasing sequence) and call [`Self::next_with_prefix`] to allocate the
/// next label of a given prefix.
#[derive(Debug, Default)]
pub struct LabelGenerator {
    next: u32,
}

impl LabelGenerator {
    /// Build a generator whose first emitted counter is `0`.
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Allocate the next fresh label of the given `prefix`.
    ///
    /// Returns `format!("{prefix}.{counter}")` and increments the counter.
    pub fn next_with_prefix(&mut self, prefix: &str) -> String {
        let n = self.next;
        self.next += 1;
        format!("{prefix}.{n}")
    }
}