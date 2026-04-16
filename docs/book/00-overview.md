# 00 — Project Overview

## The overall project

You are building a Rust compiler for a substantial subset of C. The work is incremental. Each chapter extends the compiler's language coverage, internal representations, semantic rules, or backend sophistication.

The project naturally breaks into five arcs:

1. **Chapters 1–4**: expressions, precedence, truthiness, and short-circuiting
2. **Chapters 5–8**: variables, scope, blocks, branching, and loops
3. **Chapters 9–13**: functions, globals, integer-width growth, and floating point
4. **Chapters 14–18**: pointers, arrays, strings, heap support, and aggregates
5. **Chapters 19–20**: optimization and register allocation

## The compiler pipeline

A stable mental model for the whole book is:

```text
source file
  -> preprocess (external toolchain)
  -> lex
  -> parse
  -> semantic analysis
  -> lower to IR / TACKY
  -> optimize
  -> lower to assembly-oriented form
  -> allocate registers / stack slots
  -> emit assembly
  -> assemble + link (external toolchain)
```

## The implementation strategy

Follow this order inside the repo:

- make the **driver** reliable first
- keep **frontend** code independent of codegen
- let **semantic analysis** become the type/scope authority
- let **TACKY / IR** become the optimization and lowering boundary
- let **backend** code own ABI and machine details
- treat **tests** as the acceptance oracle for each chapter

## Major repository areas

- `src/` — active Rust compiler implementation
- `tests/` — official test harness and test corpus
- `docs/book/` — chapter-first implementation guide
- `docs/specs/` — software requirements specification
- `docs/research/` — curated external references
- `src/` — active compiler code plus the placeholder skeleton for later book chapters
- `.omx/plans/` — planning artifacts that explain why this package exists

## What changes over time

Early chapters mostly change:

- lexer
- parser
- AST
- direct/simple codegen

Middle chapters mostly change:

- symbol resolution
- type checking
- stack layout
- function and global handling

Late chapters mostly change:

- richer types and layouts
- ABI-sensitive lowering
- TACKY passes
- liveness / interference
- register allocation

## What should stay stable

Even as the compiler grows, keep these stable:

- deterministic stage boundaries
- explicit intermediate outputs where useful
- reproducible build/test commands
- chapter-by-chapter traceability
- an isolated scaffold that never becomes hidden production code

## How to use the rest of this guide

- read `01-toolchain-and-workflow.md` before implementing anything
- use each chapter guide as a work checklist
- cross-check every chapter against `test-map.md` and `requirements-map.md`
- use the appendices when debugging ABI or backend issues
