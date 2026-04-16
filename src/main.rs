mod ast;
mod codegen;
mod compiler;
mod driver;
mod ir;
mod lex;
mod parse;
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
