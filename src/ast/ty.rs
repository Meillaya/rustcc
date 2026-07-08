//! Type node placeholders.

#![allow(dead_code)]

/// Placeholder type surface for the growing language subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Long,
    UnsignedInt,
    UnsignedLong,
    Double,
    Char,
    Void,
    Pointer,
    Array,
    Struct,
    Union,
    Function,
}
