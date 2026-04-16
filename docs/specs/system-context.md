# System Context

## System boundary

`rustcc` is a Rust command-line compiler for the book-sized C subset described by Nora Sandler.

It consumes C source files, performs frontend and backend compilation work, and emits diagnostics or build artifacts.

## Actors

- learner / developer
- test harness
- host toolchain (`gcc`/`clang`, assembler, linker)
- runtime environment for produced executables

## Inputs

- C translation units
- CLI flags controlling stage and feature selection
- optional helper libraries from the test suite

## Outputs

- diagnostics
- intermediate artifacts when requested
- assembly and executables for full builds

## External dependencies

- local filesystem
- host toolchain for preprocess / assemble / link steps
- platform ABI rules
