# Chapter 0 — Driver, Harness, and Phase Envelope Contract

## Purpose

Chapter 0 does not implement compiler algorithms. It locks the public contract that later chapters must preserve while the real lexer, parser, semantic analysis, TACKY lowering, backend, and toolchain calls are filled in.

## Driver option pseudocode

```text
parse args:
  stage defaults to Run
  artifact mode defaults to Executable
  stage flags (--lex, --parse, --validate, --tacky, --codegen) force StdoutOnly
  -S requests AssemblyFile over the full pipeline
  -c requests ObjectFile over the full pipeline
  optimization flags are recorded for TACKY chapter behavior
  --no-coalescing disables only coalescing, not register allocation
  linker pass-through flags such as -lm are preserved for final link
```

## Artifact matrix

```text
--lex/--parse/--validate/--tacky/--codegen -> stdout only; no .s/.o/executable
-S                                             -> <stem>.s only
-c                                             -> <stem>.o only; temporary .s deleted
default run                                    -> executable <stem>
invalid/rejected input                         -> delete stale .s/.o/executable
```

## Phase envelope pseudocode

```text
CompilerArtifacts:
  tokens_pretty?
  ast_pretty?
  typed_ast_pretty?
  tacky_pretty?
  assembly_text?

CompileOptions:
  stage
  optimization_flags
  regalloc_options
```

## TACKY pretty-output contract

```text
for each function in source order:
  print function signature
  print stable labels/basic blocks
  print one instruction per line
  include %tN temporaries, constants, type annotations when semantic behavior depends on type
  include explicit terminators
print globals/static data in a stable section when needed
```

## Verification target

This chapter document is pseudocode guidance only. After you fill in the real Rust code, Chapter 0 is complete when the targeted driver/config tests and `cargo test` pass.
