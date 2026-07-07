// `lvalue_name` and `eval_values` become unused after W0-T6 deletes the old
// monolithic validate pass; their real consumers land in wave 6+ resolve and
// type checking, so the `dead_code` lint is allowed crate-wide for now.
#![allow(dead_code)]

mod ast;
mod codegen;
mod compiler;
mod driver;
mod ir;
mod lex;
mod parse;
mod pipeline;
mod semantics;
mod support;
mod toolchain;
mod util;

fn main() {
    if let Err(err) = driver::run_from_env() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
