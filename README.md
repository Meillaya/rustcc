# rustcc

## A C Compiler written in Rust

Based on Writing a C Compiler by Nora Sandler

## Usage:

`cargo run rustcc <input_file.c>`

For running each stage separately use the following flags
options:
```  
-h, --help      Display a help message
-l, --lex       Print token after lexical analysis
-p, --parse     Parse the input file and display the AST
-cg, --codegen  Print generated assembly code
--all           Enable all output phases
  ```

## Running Tests

This project uses the official test suite from [nlsandler/writing-a-c-compiler-tests](https://github.com/nlsandler/writing-a-c-compiler-tests).

First, build the compiler:
```bash
cargo build --release
```

Then run tests (requires Python 3.8+ and `gcc` on PATH):
```bash
# Test chapters 1 through N
./tests/test_compiler ./target/release/rustcc --chapter N

# Test only the latest chapter (skip earlier ones)
./tests/test_compiler ./target/release/rustcc --chapter N --latest-only

# Skip invalid test cases (only run valid programs)
./tests/test_compiler ./target/release/rustcc --chapter N --skip-invalid

# Stop on first failure
./tests/test_compiler ./target/release/rustcc --chapter N -f

# Include extra credit features
./tests/test_compiler ./target/release/rustcc --chapter N --bitwise --compound --increment
```

## References:

- [Nora Sandler's Blog](https://norasandler.com/2017/11/29/Write-a-Compiler.html): A blog post that explains parts of the compiler in detail.
- [Writing a C Compiler](https://github.com/nlsandler/nqcc2): A repository containing the code for the blog post in OCaml.
