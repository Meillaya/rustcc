//! AST type surface for chapters 1-15 of the book.
//!
//! The type system grows chapter by chapter.  Chapter 11 introduced
//! `long`; chapter 12 added `unsigned` (signedness is tracked as a
//! leaf property).  Chapter 13 added `double` as a leaf and a few
//! flags for the type system.  Chapters 14-15 made the type system
//! recursive so that `int *p` and `int a[10]` can be represented.
//!
//! The variants kept flat (`Int`, `Long`, ...) carry no payload; the
//! recursive variants (`Pointer`, `Array`) wrap a `Box<Type>` so the
//! enum is finite-sized regardless of nesting depth.  We derive
//! `PartialEq`, `Eq`, and `Hash` so the type can be used as a
//! `TypeEnv` key (the env maps variable names to their declared
//! type).

#![allow(dead_code)]

/// Concrete C types supported through chapter 15.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Long,
    UnsignedInt,
    UnsignedLong,
    Double,
    Char,
    SignedChar,
    UnsignedChar,
    Void,
    /// `T *` — pointer to a value of type `T`.
    Pointer(Box<Type>),
    /// `T[N]` — fixed-size array of `T`.  `size = None` marks an
    /// incomplete array (`int a[]`); chapter 15's `subscript.c`
    /// only requires the fixed-size form.
    Array {
        element: Box<Type>,
        size: Option<usize>,
    },
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
                | Type::SignedChar
                | Type::UnsignedChar
        )
    }

    /// Returns true for signed integer types.  Mirrors OCaml
    /// `Type_utils.is_signed` for the chapter-12 surface.
    pub fn is_signed(self) -> bool {
        matches!(self, Type::Int | Type::Long | Type::Char | Type::SignedChar)
    }

    /// Returns true for unsigned integer types.  Mirrors OCaml
    /// `Type_utils.is_unsigned` for the chapter-12 surface.
    pub fn is_unsigned(self) -> bool {
        matches!(
            self,
            Type::UnsignedInt | Type::UnsignedLong | Type::UnsignedChar
        )
    }

    /// Returns true when the type is a pointer.
    pub fn is_pointer(self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    pub fn decay(self) -> Type {
        match self {
            Type::Array { element, .. } => Type::Pointer(element),
            other => other,
        }
    }

    /// Returns true when the type is a (complete) array.
    pub fn is_array(self) -> bool {
        matches!(self, Type::Array { size: Some(_), .. })
    }

    /// Returns true when the type is a complete object (not a
    /// function / incomplete array / void).  Mirrors OCaml
    /// `Type_utils.is_complete` for the chapter-14 surface.
    pub fn is_complete(self) -> bool {
        match self {
            Type::Array { size: None, .. } | Type::Void => false,
            Type::Array {
                element,
                size: Some(_),
            } => element.is_complete(),
            Type::Pointer(_) => true,
            _ => true,
        }
    }

    /// Returns the size of the type in bytes.  Mirrors the size
    /// table in the book (int = 4, long = 8, ...).  Used by the
    /// chapter-11+ lowerer to decide between 32-bit and 64-bit
    /// codegen and by `replace_pseudos` to size stack slots.
    pub fn size(self) -> i64 {
        match self {
            Type::Char | Type::SignedChar | Type::UnsignedChar => 1,
            Type::Int | Type::UnsignedInt => 4,
            Type::Long | Type::UnsignedLong => 8,
            Type::Double => 8,
            Type::Pointer(_) => 8,
            Type::Array {
                element,
                size: Some(n),
            } => element.size() * n as i64,
            Type::Array { size: None, .. } | Type::Void => 0,
        }
    }
}
