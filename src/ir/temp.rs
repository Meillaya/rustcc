//! Temporary-variable identifiers.
//!
//! Mirrors OCaml `unique_ids.ml` pattern: a monotonically increasing counter
//! is wrapped in a typed newtype so the AST and IR cannot accidentally use a
//! raw `u32` for a temporary.  The generator owns the counter so each
//! lowering (or optimization pass) gets a fresh, monotonic sequence.

#![allow(dead_code, unused_variables)]

/// A typed temporary identifier.
///
/// Distinct from [`crate::ir::tacky::Var`] (a `String`) so codegen and
/// optimizations can pattern-match on "real temporaries" vs "user variables"
/// without scanning names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TempId(pub u32);

/// Monotonic generator for fresh [`TempId`]s.
///
/// Construct one per lowering or per optimization pass and call
/// `next()` to allocate the next identifier.  `next()` is `&mut self` to
/// keep the borrow rules explicit at every call site.
#[derive(Clone, Debug, Default)]
pub struct TempIdGenerator {
    next: u32,
}

impl TempIdGenerator {
    /// Build a generator whose first emitted id is `0`.
    pub fn new() -> Self {
        Self { next: 0 }
    }

    /// Allocate the next fresh temporary id.
    pub fn next(&mut self) -> TempId {
        let id = self.next;
        self.next += 1;
        TempId(id)
    }
}
