# rustcc

## A C Compiler written in Rust

Based on Writing a C Compiler by Nora Sandler.

## Usage

Build the compiler:

```bash
cargo build --release
```

Compile a C file with the `rustcc` binary:

```bash
target/release/rustcc [stage/options] <input.c>
```

Common examples:

```bash
# Compile and link ./examples/hello.c to ./examples/hello
target/release/rustcc examples/hello.c

# Stop after assembly or object generation
target/release/rustcc -S examples/hello.c
target/release/rustcc -c examples/hello.c

# Print intermediate compiler output to stdout
target/release/rustcc --lex examples/hello.c
target/release/rustcc --parse examples/hello.c
target/release/rustcc --validate examples/hello.c
target/release/rustcc --tacky examples/hello.c
target/release/rustcc --codegen examples/hello.c
```

`rustcc` accepts one `.c` input file. By default it preprocesses, compiles,
assembles, and links an executable next to the input path, using the input file
stem as the output name. Stage-stop flags (`--lex`, `--parse`, `--validate`,
`--tacky`, `--codegen`) print that stage's output to stdout instead of writing
object/executable artifacts. `-S` writes `<input-stem>.s`; `-c` writes
`<input-stem>.o`.

Supported compiler flags:

```text
-h, --help                       Display usage text (currently exits nonzero)
-l, --lex                        Print tokens after lexical analysis
-p, --parse                      Parse the input file and print the AST
--validate                       Print the typed/validated AST
--tacky                          Print TACKY IR
-cg, --codegen                   Print generated assembly
--all, --run                     Run the full compile/link pipeline (default)
-S                               Write assembly output
-c                               Write object output
--fold-constants                 Enable constant folding
--eliminate-unreachable-code     Enable unreachable-code elimination
--propagate-copies               Enable copy propagation
--eliminate-dead-stores          Enable dead-store elimination
--no-coalescing                  Disable register coalescing
-lm                              Forward libm to the linker
```

## Running Tests

This project uses the official test suite from [nlsandler/writing-a-c-compiler-tests](https://github.com/nlsandler/writing-a-c-compiler-tests).

The chapter-selection flags belong to the Python test harness,
`tests/test_compiler`; they are not accepted by `rustcc` itself.

First, build the compiler:

```bash
cargo build --release
```

Then run chapter gates (requires Python 3.8+ and `gcc` on PATH):

```bash
# Test chapters 1 through N through the harness
./tests/test_compiler ./target/release/rustcc --chapter N

# Test only chapter N (skip earlier chapters)
./tests/test_compiler ./target/release/rustcc --chapter N --latest-only

# Skip invalid test cases temporarily while developing a backend feature
./tests/test_compiler ./target/release/rustcc --chapter N --skip-invalid

# Stop on first failure
./tests/test_compiler ./target/release/rustcc --chapter N -f

# Include extra credit feature groups for chapters that support them
./tests/test_compiler ./target/release/rustcc --chapter N --bitwise --compound --increment
```

See `docs/book/test-map.md` for the maintained chapter-by-chapter harness
commands, including Chapter 19 optimization and Chapter 20 register-allocation
flags.

## References

- [Nora Sandler's Blog](https://norasandler.com/2017/11/29/Write-a-Compiler.html): A blog post that explains parts of the compiler in detail.
- [Writing a C Compiler](https://github.com/nlsandler/nqcc2): A repository containing the code for the blog post in OCaml.
- [rlox](https://github.com/Meillaya/rlox): A complete implementation of the lox programming language interpreter in Rust.
