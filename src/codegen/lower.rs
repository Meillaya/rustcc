//! Native assembly text for the early constant-return backend.
//!
//! The current native path has already evaluated the whole single-function
//! program to an `i32`, so this backend emits the smallest System V x86-64
//! function body needed by the tests.  Keeping it as a pure `String` function
//! makes ownership simple: callers receive assembly text with no borrowed state
//! and no hidden filesystem side effects.

pub(crate) fn emit_native_constant_function(function_name: &str, return_value: i32) -> String {
    format!(
        "    .globl {name}\n{name}:\n    movl ${value}, %eax\n    ret\n",
        name = function_name,
        value = return_value
    )
}
