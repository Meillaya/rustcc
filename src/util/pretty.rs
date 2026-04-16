//! Placeholder pretty-printing surfaces for debugging and stage output.

#![allow(dead_code)]

/// Marker trait for future pretty-printable compiler structures.
pub trait PrettyPrint {
    fn pretty_print(&self) -> String;
}
