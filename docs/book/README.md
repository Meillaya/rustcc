# rustcc Book Guide

This directory is the main, chapter-first guide for implementing the compiler in this repository by following Nora Sandler's *Writing a C Compiler* from start to finish.

## What this guide is for

Use this guide when you want a structured path from:

- an empty or partial Rust compiler project
- through each chapter's language features and compiler passes
- to a working compiler that can pass the bundled official test suite

This guide is intentionally:

- **book-locked** to the local PDF in `docs/Writing a C Compiler - Sandler, Nora.pdf`
- **test-aware** with explicit references to the bundled Nora test suite in `tests/`
- **implementation-guiding** rather than implementation-writing
- **Rust-oriented** even though the book presents language-agnostic pseudocode

## Source-of-truth note

Chapter numbering and chapter names are locked to:

1. `docs/Writing a C Compiler - Sandler, Nora.pdf`
2. `tests/README.md`
3. the directory tree under `tests/tests/chapter_*`

If those sources ever disagree, update `chapter-lock.md` before changing any chapter-facing docs.

## Recommended reading order

1. `00-overview.md`
2. `01-toolchain-and-workflow.md`
3. `stage-crosswalk.md`
4. `chapter-lock.md`
5. `ch01-minimal-compiler.md` through `ch20-register-allocation.md`
6. the appendices for debugging and quick-reference work

## Guide layout

- `00-overview.md` — big-picture map of the full compiler project
- `01-toolchain-and-workflow.md` — environment, commands, test loop, and debugging workflow
- `ch01-*.md` to `ch20-*.md` — chapter-by-chapter implementation guidance
- `chapter-lock.md` — canonical chapter baseline
- `stage-crosswalk.md` — mapping from chapter-first docs to the repo's stage-oriented expectations
- `chapter-map.md` — chapter summary table
- `test-map.md` — chapter-to-test-suite map
- `scaffold-map.md` — chapter-to-scaffold map
- `requirements-map.md` — chapter-to-SRS map
- `templates/` — authoring templates used by this package
- `appendices/` — debugging and reference material

## Quick test loop

```bash
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 1 --latest-only
./tests/test_compiler ./target/release/rustcc --chapter 5
./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
```

## Package contract

When you work through this guide, keep these rules in mind:

- build the compile driver first, then grow the compiler in chapter order
- keep phases separated: driver, lexer, parser, semantics, IR, backend, optimizer
- preserve old behavior when adding new features
- prefer explicit tests and traceability over clever abstractions
- use the in-`src/` skeleton for structure, but keep the placeholder modules non-implementational until their chapters begin
