# 01 — Toolchain and Workflow

## Required tools

You need:

- Rust toolchain (`cargo`, `rustc`)
- Python 3.8 or later
- `gcc` on `PATH` (on macOS this may resolve to Clang, which is fine for the test suite)

## Local smoke checks

Run these first:

```bash
cargo check
cargo build --release
./tests/test_compiler --check-setup
```

If `--check-setup` fails, fix the environment before blaming your compiler.

## Core development loop

Use this loop for nearly every chapter:

1. read the relevant chapter in the PDF
2. read the matching guide page in `docs/book/`
3. inspect the matching tests in `tests/tests/chapter_N/`
4. implement the smallest slice that should pass
5. run chapter-scoped tests
6. inspect failures, fix, rerun
7. only then move to the next feature

## Important test-harness commands

Run all tests up through a chapter:

```bash
./tests/test_compiler ./target/release/rustcc --chapter N
```

Run only the latest chapter:

```bash
./tests/test_compiler ./target/release/rustcc --chapter N --latest-only
```

Skip invalid tests while backend work is in progress:

```bash
./tests/test_compiler ./target/release/rustcc --chapter N --skip-invalid
```

Stop on first failure:

```bash
./tests/test_compiler ./target/release/rustcc --chapter N --failfast
```

Require specific rejection codes for chapter 1 frontend failures:

```bash
./tests/test_compiler ./target/release/rustcc --chapter 1 --expected-error-codes 1 2
```

Preserve assembly on failing valid programs:

```bash
./tests/test_compiler ./target/release/rustcc --chapter N --keep-asm-on-failure
```

## Stage-limited workflow

The harness supports these stages:

- `lex`
- `parse`
- `validate`
- `tacky`
- `codegen`
- `run`

Use them to isolate failures:

```bash
./tests/test_compiler ./target/release/rustcc --chapter 5 --stage validate
./tests/test_compiler ./target/release/rustcc --chapter 19 --stage tacky
./tests/test_compiler ./target/release/rustcc --chapter 20 --stage codegen --no-coalescing
```

## Extra-credit switches

These feature switches appear in the official harness:

- `--bitwise`
- `--compound`
- `--increment`
- `--goto`
- `--switch`
- `--nan`
- `--union`
- `--extra-credit`

Optimization and regalloc switches:

- `--fold-constants`
- `--eliminate-unreachable-code`
- `--propagate-copies`
- `--eliminate-dead-stores`
- `--int-only`
- `--no-coalescing`

## Practical workflow advice

### Frontend chapters

Prefer this order:

```text
lex -> parse -> validate -> codegen -> run
```

### Backend chapters

Prefer this order:

```text
validate -> tacky -> codegen -> run
```

### Optimization chapters

Prefer this order:

```text
tacky -> optimized tacky -> codegen -> run
```

### Register allocation chapter

Prefer this order:

```text
codegen without coalescing -> run -> codegen with coalescing -> run
```

## Debugging habits that pay off

- keep representative failing inputs in a scratch folder
- inspect generated assembly for every backend bug
- use `objdump`, `readelf`, and `gdb`/`lldb` when runtime behavior is surprising
- compare your generated shapes to the ABI rules, not only to intuition
- when a new chapter breaks old tests, suspect a regression in shared logic first

## A good release-candidate definition

A chapter is not truly done until:

- latest chapter tests pass
- earlier chapters still pass
- invalid cases reject correctly
- valid cases run correctly
- no accidental stage leakage occurs
- the relevant docs/maps stay up to date
