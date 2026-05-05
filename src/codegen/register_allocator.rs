//! Register-allocation ownership surface for the final backend chapters.
//!
//! The current implementation still relies on the system C bridge for Chapter
//! 20 register-allocation behavior.  This module intentionally reserves the
//! native allocator boundary without pretending to perform allocation yet.

#![allow(dead_code)]

/// Future register-allocation façade.
#[derive(Debug, Default)]
pub struct RegisterAllocator;
