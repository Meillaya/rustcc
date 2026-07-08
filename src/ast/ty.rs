//! Type node placeholders.

#![allow(dead_code)]

/// Placeholder type surface for the growing language subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Type {
    /// Returns true for integer types (signed + unsigned, all widths).
    pub fn is_integer(self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Long
                | Type::UnsignedInt
                | Type::UnsignedLong
                | Type::Char
        )
    }

    /// Returns the size of the type in bytes.  Mirrors the size table
    /// in the book (int = 4, long = 8, ...).  Used by the chapter-11+
    /// lowerer to decide between 32-bit and 64-bit codegen and by
    /// `replace_pseudos` to size stack slots.
    pub fn size(self) -> i64 {
        match self {
            Type::Char => 1,
            Type::Int | Type::UnsignedInt => 4,
            Type::Long | Type::UnsignedLong => 8,
            _ => 8,
        }
    }
}
