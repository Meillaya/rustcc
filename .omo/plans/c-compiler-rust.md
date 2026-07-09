# rustcc: Complete C Compiler in Rust (All 20 Chapters + All Extras)

## TL;DR

**Quick Summary**: Complete `/home/mei/projects/rustcc` as a strict 1:1 Rust port of Nora Sandler's OCaml reference compiler (`nqcc2/`), implementing all 20 chapters of *Writing a C Compiler* with native x86-64 System V AMD64 codegen. All 7 extra-credit features (bitwise, compound assignment, increment/decrement, goto, switch, NaN-aware floats, union) and chapter 19's four optimization passes plus chapter 20's graph-coloring register allocation are in scope.

**Deliverables**:
- A native Rust compiler binary `rustcc` that produces executable x86-64 Linux ELF objects via `gcc` assembly+link
- End-to-end pipeline: read `.c` -> `gcc -E -P` preprocess -> lex -> parse -> resolve -> label_loops -> typecheck -> AST->TACKY -> TACKY opt -> TACKY->assembly AST -> assembly fixup -> replace pseudos -> emit `.s`
- All 20 chapters' language features pass the official `nlsandler/writing-a-c-compiler-tests` harness with the chapter's required extras

**Estimated Effort**: XL - multi-week, ~70 tasks across 23 sequential waves with strict per-chapter gates
**Parallel Execution**: PARTIAL - strong intra-chapter parallelism; strict inter-chapter seriality per locked decision D
**Critical Path**: Wave 0 (foundation rewrite) -> Wave 1 (lexer) -> Waves 2-9 (chapters 1-8) -> Wave 10 (chapter 9 functions - biggest pivot) -> Wave 11 (chapter 10 globals) -> Wave 12 (chapter 11 long - foundation infra) -> Waves 13-17 (chapters 12-16) -> Wave 19 (chapter 18 structs + union extra) -> Wave 20 (chapter 19 optimizations) -> Wave 21 (chapter 20 register allocation) -> Wave 22 (integration/polish) -> Final Wave (F1-F4)

---

## Context

### Original Request

Complete the existing Rust C compiler at `/home/mei/projects/rustcc` according to every chapter of Nora Sandler's book *Writing a C Compiler*, including all "extra" (extra-credit) implementations and including codegen. The book has 20 chapters plus seven distinct extra-credit feature groups; the project must end with the official `nlsandler/writing-a-c-compiler-tests` harness passing end-to-end on every chapter.

### Interview Summary (Key Decisions - Locked)

- **A. Reference source**: STRICT 1:1 MIRROR of the OCaml reference at `/home/mei/projects/rustcc/nqcc2/`. Port modules module-for-module preserving names, signatures, and structure. Allow Rust-idiomatic substitutions for OCaml functors -> generics+traits, mutable global state -> context structs/atomics, exceptions -> `Result`.
- **B. Preprocessor**: Shell to `gcc -E -P` via the driver (mirrors `nqcc2/bin/main.ml` exactly).
- **C. Existing scaffolding fate**: TEAR OUT the interpreter (`src/ir/control_flow.rs`) and the system-C bridge (`src/codegen/emit.rs`, `src/support/source.rs` heuristics, `src/toolchain.rs::evaluate_with_system_cc*`/`system_c_syntax_check*`/`system_c_to_assembly*` helpers, the `compile_with_system_cc_frontend` function in `src/compiler.rs`). Fix the broken `src/compiler.rs:1` stray-`w` syntax error. Rebuild everything natively.
- **D. Test gating rhythm**: STRICT PER-CHAPTER. After each chapter's tasks complete, run `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only` plus the chapter's required extras per `docs/book/test-map.md`. Move to the next chapter only when that gate is green. Also run `--chapter N` (without `--latest-only`) periodically to catch regressions.

### Research Findings

- The book has **20 chapters**, not 16. Chapters 19-20 are optimization capstones. Seven extras (bitwise ch3+, compound ch5+, increment ch5+, goto ch6+, switch ch8+, nan ch13+, union ch18+) are gated by `--bitwise --compound --increment --goto --switch --nan --union` flags.
- Codegen target is **x86-64 System V AMD64**. Integer/pointer args in `%rdi %rsi %rdx %rcx %r8 %r9`; float args in `%xmm0-%xmm7`; `gcc` is used as assembler+linker.
- `enum` is **out of scope** (book does not include it; no tests).
- `nqcc2` includes 21 commits (chapters 1-20, with chapter 20 split into 20a "without coalescing" and 20 "with conservative coalescing") and matches the test commands in `docs/book/test-map.md`.
- Preprocessor is delegated to `gcc -E -P` in `nqcc2/bin/main.ml:30-34`, not implemented in the compiler itself.

### Metis Review (key applied findings)

- The current `src/compiler.rs` calls `evaluate_program` (interpreter) and `source.rs` heuristics (system-C gate). Tearing out the interpreter is not a clean `rm`: requires rewriting `compiler.rs` first, then deleting the interpreter module. Wave 0 addresses this ordering.
- `src/ir/tacky.rs` (31 LOC) is **not** book-faithful TACKY; replace with proper TACKY matching `nqcc2/lib/tacky.ml` (99 LOC).
- `src/ast` is single-function only. Multi-function reshape at chapter 9 is the largest pivot. Waves 0 and 10 prepare for it.
- Chapter 13 introduces a second register class (`double`/XMM); chapter 20's register allocator runs **twice** (GP and XMM). Plan in waves 14, 21.
- Chapter 18's struct ABI classification (Integer/SSE/Memory eightbytes) is the most complex codegen logic. Wave 19.
- Chapter 19's `cfg.ml` is a functor reused by chapter 20's liveness. Rust port needs generic `Cfg<T>` with trait bounds.
- Several OCaml files port non-trivially due to mutable state, functors, and Zarith: `cfg.ml`, `regalloc.ml`, `typecheck.ml`, `codegen.ml`, `parse.ml`, `tacky_gen.ml`, `disjoint_sets.ml`, `backward_dataflow.ml`, `constant_folding.ml`. Wave assignments account for these costs.

---

## Work Objectives

### Core Objective

Build a complete Rust C compiler that:
1. Reads any valid C source up to chapter 18 of the book (with all 7 extras),
2. Compiles to x86-64 System V AMD64 assembly (Linux target only),
3. Passes every test in `nlsandler/writing-a-c-compiler-tests` for chapters 1-20, with all extras,
4. Embeds the book's prescribed optimization passes (constant folding, copy propagation, dead store elimination, unreachable code elimination),
5. Performs register allocation with graph coloring (with and without conservative coalescing),
6. Mirrors the OCaml reference in `nqcc2/` module-for-module.

### Concrete Deliverables

- **rustcc binary**: A working `cargo build --release` artifact at `target/release/rustcc`.
- **Native backend**: A working `TACKY -> Assembly AST -> .s` pipeline (replacing current interpreter shim and system-C sanitizer).
- **Module structure mirroring `nqcc2/lib/`**: `src/lex/`, `src/parse/`, `src/ast/`, `src/semantics/` (resolve+label_loops+typecheck), `src/ir/` (TACKY+CFG+opts), `src/codegen/` (asm+codegen+fixup+regalloc+replace+emit).
- **Test invocation**: `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only` plus extras per `docs/book/test-map.md`.
- **End-to-end execution**: Each `.c` test compiles to an executable producing the harness-expected output or rejecting with the expected exit code.
- **`docs/COACHING_LOG.md`** updated with each chapter's gate as having been run.

### Definition of Done

- [ ] `cargo build --release` produces `./target/release/rustcc` with zero warnings.
- [ ] For each `N` in `1..=18`, `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only --extra-credit` reports every test passing.
- [ ] For each `N` in `1..=18`, core (no extras) also still passes (regression check).
- [ ] For chapter 19, each of `--fold-constants`, `--eliminate-unreachable-code`, `--propagate-copies`, `--eliminate-dead-stores`, and the default all-optimizations run passes `--chapter 19 --latest-only`.
- [ ] For chapter 20, both `--chapter 20 --latest-only --no-coalescing` and `--chapter 20 --latest-only` (with coalescing) pass.
- [ ] The existing `cargo test` unit tests (`src/driver.rs` + `src/compiler.rs`) still pass.
- [ ] All scaffold files from prior scaffolding phase (interpreter, system-C bridge, source heuristics) are deleted from `src/`.

### Must Have

- All 20 chapters' language features per `nqcc2`.
- All 7 extras (bitwise, compound, increment, goto, switch, nan, union).
- Chapter 19's 4 named optimization passes only - no extras (no inlining, strength reduction, etc.).
- Chapter 20's graph-coloring register allocation with both `--no-coalescing` and the default-with-coalescing.
- x86-64 System V AMD64 ABI (Linux target only).
- End-to-end compilation via `gcc` as the assembler/linker.
- Preprocessing via `gcc -E -P`.
- The official `test_compiler` Python harness as the only verification path for compiler correctness.
- Tests `tests/` directory contents remain unchanged.

### Must NOT Have (Guardrails)

- `enum`, struct bitfields, switch-on-string, `longjmp`/`setjmp`, variadic functions, `typedef`, `_Bool`, `long long`, `long double`, `restrict`, `volatile`, `inline`, `__attribute__`, statement expressions, nested functions.
- Native macOS, x86-64 Microsoft ABI, ARM64, RISC-V (Linux x86-64 System V only).
- Custom preprocessor, assembler, or linker.
- Optimization passes beyond the 4 named in chapter 19.
- Sign/zero extension beyond what chapter 12 specifies.
- Error message spans, suggestions, fancy diagnostics (harness checks exit codes only).
- Procedural macros or lifetime-erasure hacks for OCaml-functor pattern; use generics+traits directly.
- Visitor patterns, fluent builders, generic AST types; use enums matching the book's pattern matching.
- Custom test framework, snapshot/golden tests, Rust unit tests for compiler phases.
- Debug info (`-g`, DWARF), PIE/PIC, stack protectors, sanitizer passes.
- Anything not in the book.

---

## Verification Strategy (MANDATORY)

### Test Decision

- **Infrastructure exists**: YES (the official `nlsandler/writing-a-c-compiler-tests` is at `tests/`; the `test_compiler` Python harness at `tests/test_compiler`).
- **Automated tests**: STRICT PER-CHAPTER via the official `test_compiler` Python harness, gated per locked decision D. Each chapter closes only after its gate is green.
- **Framework**: `tests/test_compiler` (Python 3.8+) invocations:
  - `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only` (core)
  - `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only --bitwise --compound --increment --goto --switch --nan --union` (all extras cumulatively)
  - For chapters 19-20, follow dedicated flags per `docs/book/test-map.md`.
- **If a chapter fails**: do not proceed. Investigate, fix, re-run.
- **No Rust unit tests for compiler phases**. Existing 9 in `src/driver.rs` + `src/compiler.rs` cover CLI/orchestration glue.
- **Regression cadence**: After every 3rd chapter, also run `--chapter N` (no `--latest-only`) to catch regressions.

### QA Policy

Each task MUST include agent-executed QA scenarios. Evidence saved to `.omo/evidence/task-{N}-{scenario-slug}.{ext}`.

For each chapter's test invocation, the executing agent MUST:
1. Build: `cargo build --release`
2. Run: `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only [--extras per docs/book/test-map.md]`
3. Save: full stdout+stderr to `.omo/evidence/task-{N}-chapter-gate.txt`
4. Save: count of `pass`/`fail` lines and exit code.
5. If `fail` count non-zero, task is **not** complete - fix and re-run.

The orchestrator verifying task completion will inspect `.omo/evidence/task-{N}-chapter-gate.txt` and confirm `fail=0` and exit code 0/expected.

---

## Execution Strategy

### Parallel Execution Waves

23 sequential waves. Inter-wave seriality strict. Intra-wave parallelism maximum.

| Wave | Focus | Tasks | Gate |
|---:|---|---:|---|
| 0 | Foundation rewrite, OCaml-mirror scaffold, fix broken state | 7 | `cargo build --release` zero warnings |
| 1 | Lexer (port `nqcc2/lib/lex.ml` + `tokens.ml`) | 1 (cannot be split: the lexer is one cohesive module; splitting into tokens-vs-scanner would force every subsequent change to rewire both halves and contradict the book's single-file structure) | `cargo build --release`; round-trip lex on chapter 1 token sample |
| 2 | Chapter 1 minimal compiler (return int) | 3 | `--chapter 1 --latest-only` |
| 3 | Chapter 2 unary operators (`-`, `~`) | 2 | `--chapter 2 --latest-only` |
| 4 | Chapter 3 binary operators + `--bitwise` extra | 3 | `--chapter 3 --latest-only --bitwise` |
| 5 | Chapter 4 logical and relational operators | 1 (cannot be split: chapter 4's three sub-features - relational ops, equality ops, short-circuit `&&`/`\|\|` lowering - all share the same precedence-climbing parser entry and the same boolean-normalization rewrite in `lower.rs::ast_to_tacky`; splitting them into 3 tasks would require 3 separate parser modifications that would merge-conflict on each other) | `--chapter 4 --latest-only --bitwise` |
| 6 | Chapter 5 local variables + `--compound` + `--increment` extras | 2 | `--chapter 5 --latest-only --bitwise --compound --increment` |
| 7 | Chapter 6 if/else/ternary + `--goto` extra | 2 | `--chapter 6 --latest-only --bitwise --compound --increment --goto` |
| 8 | Chapter 7 compound statements | 2 | `--chapter 7 --latest-only --compound --goto` |
| 9 | Chapter 8 loops + `--switch` extra | 3 | `--chapter 8 --latest-only --compound --increment --goto --switch` |
| 10 | Chapter 9 functions (multi-function AST pivot) | 4 | `--chapter 9 --latest-only --bitwise --compound --increment --goto --switch` |
| 11 | Chapter 10 file-scope variables, linkage | 2 | `--chapter 10 --latest-only` |
| 12 | Chapter 11 `long` 64-bit (foundation infra) | 2 | `--chapter 11 --latest-only` |
| 13 | Chapter 12 unsigned integers | 2 | `--chapter 12 --latest-only` |
| 14 | Chapter 13 `double` floats + XMM + `--nan` extra | 3 | `--chapter 13 --latest-only --nan` |
| 15 | Chapter 14 pointers | 2 | `--chapter 14 --latest-only` |
| 16 | Chapter 15 arrays and pointer arithmetic | 2 | `--chapter 15 --latest-only` |
| 17 | Chapter 16 characters and string literals | 2 | `--chapter 16 --latest-only` |
| 18 | Chapter 17 dynamic memory support (`void`, `sizeof`) | 2 | `--chapter 17 --latest-only` |
| 19 | Chapter 18 structs + `--union` extra | 3 | `--chapter 18 --latest-only --union` |
| 20 | Chapter 19 optimization passes (CFG + 4 named) | 5 | `--chapter 19 --latest-only --fold-constants --eliminate-unreachable-code --propagate-copies --eliminate-dead-stores` (each individually + default) |
| 21 | Chapter 20 register allocation | 5 | `--chapter 20 --latest-only --no-coalescing` and `--chapter 20 --latest-only` (with coalescing) |
| 22 | Final integration & polish | 3 | All chapter gates green; full regression green |
| Final | F1-F4 parallel reviewers + user OK | 4 | All four APPROVE + user OK |

### Agent Dispatch Summary

| Wave | Profile mix |
|---:|---|
| 0 | 4 quick + 3 unspecified-high |
| 1 | 1 unspecified-high |
| 2 | 2 unspecified-high + 1 deep |
| 3-9 | mostly unspecified-high with deep for assembly-emission |
| 10 | 3 deep + 1 unspecified-high (chapter 9 is the biggest pivot) |
| 11-19 | unspecified-high and deep distributed |
| 20 | 4 deep + 1 unspecified-high (opt passes are subtle) |
| 21 | 5 deep (regalloc is hard) |
| 22 | 2 unspecified-high + 1 quick |
| Final | 1 oracle + 2 unspecified-high + 1 deep |

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> **FORMAT**: Task labels use bare numbers: `1.`, `2.`, etc.
> Final Wave labels use `F1.`, `F2.`, etc.
> Each task MUST have: Recommended Agent Profile + Parallelization info + References to specific `nqcc2/lib/*.ml` files for OCaml mirror + QA Scenarios.
> **A task WITHOUT QA Scenarios is INCOMPLETE.**

Tasks numbered globally 1...N. Wave prefix in title.

- [x] 1. W0-T1: Fix compiler.rs broken syntax and strip interpreter/bridge call sites

  **What to do**:
  - Open `src/compiler.rs:1`. Remove the stray `w` so the file becomes `//! Public boundary for compiler internals.`
  - Replace the entire `compile()` function in `src/compiler.rs:77` with a real pipeline skeleton: lex -> parse -> resolve -> label_loops -> typecheck -> ast_to_tacky -> tacky_opt -> tacky_to_asm -> asm_fixup -> replace_pseudos -> emit. Each stage returns `Err(anyhow!("chapter N: pipeline stage X not yet wired"))` until landed.
  - Remove the entire `compile_with_system_cc_frontend` function.
  - Remove all 9 `use crate::support::source::{...}` imports and all 7 `source_has_*`/`should_defer_parse_to_system_frontend`/`semantic_error_that_should_parse`/`likely_parse_error`/`likely_struct_or_union_parse_error` references.
  - Remove `use crate::toolchain::{evaluate_with_system_cc, system_c_syntax_check, system_c_to_assembly}`.
  - Remove `use crate::ir::evaluate_program` and the entire `if source.contains("long") || ...` gate at `compiler.rs:86-104`.
  - Keep the 5 `#[cfg(test)]` unit tests at `src/compiler.rs:252` passing.

  **Must NOT do**: Do not change the public API of `compile()` (signature, `CompileOptions`, `CompilerArtifacts`); do not delete any of the 9 unit tests.

  **Recommended Agent Profile**: Category `quick`. Mechanical rewiring; no algorithmic complexity.

  **Parallelization**: Sequential. Prerequisite for all other Wave-0 tasks. Blocked By: none. Blocks: W0-T2..T7.

  **References** (mirror what `nqcc2/bin/main.ml` looks like after porting to Rust):
  - OCaml driver entry: `nqcc2/bin/main.ml`
  - OCaml compile orchestrator: `nqcc2/lib/compile.ml`
  - Current Rust facade: `src/compiler.rs:77-110` (interpreter gate to delete); `src/compiler.rs:227-294` (system-C region to delete)
  - Current CLI enum: `src/driver.rs:36-50`

  **Acceptance Criteria**:
  - [ ] `cargo check --release` zero errors.
  - [ ] `grep -rn "evaluate_program" src/` zero matches.
  - [ ] `grep -rn "compile_with_system_cc_frontend" src/` zero matches.
  - [ ] `grep -rn "should_defer_parse_to_system_frontend\|source_has_\|likely_parse_error\|semantic_error_that_should_parse" src/` zero matches.
  - [ ] `grep -n "evaluate_with_system_cc\|system_c_syntax_check\|system_c_to_assembly" src/toolchain.rs` zero matches.
  - [ ] `cargo test --release` passes all 9 pre-existing unit tests.

  **QA Scenarios**:

  ```
  Scenario: Build is clean after interpreter and bridge references are removed
    Tool: Bash (cargo)
    Steps:
      1. cargo build --release
      2. Assert exit_code == 0, stderr empty
    Failure Indicators: errors citing `expected ;`, `cannot find value`, `unresolved import` -> fix imports
    Evidence: .omo/evidence/task-1-cargo-build-clean.txt

  Scenario: Existing unit tests still pass after refactor
    Tool: Bash (cargo)
    Steps:
      1. cargo test --release
      2. Assert 9 tests pass (4 driver + 5 compiler), 0 fail
    Failure Indicators: test fail referencing renamed/removed function -> fix the test or restore the function
    Evidence: .omo/evidence/task-1-cargo-test.txt
  ```

- [x] 2. W0-T2: Delete the runtime interpreter (`src/ir/control_flow.rs`)

  **What to do**:
  - `rm src/ir/control_flow.rs` (the 262-LOC runtime evaluator; not book-faithful).
  - Update `src/ir/mod.rs`: remove `pub mod control_flow;` and `pub use control_flow::evaluate_program;`. Add comment `// No interpreter; the IR is consumed only by codegen.`
  - Verify `grep -rn "control_flow" src/` returns no `.rs` matches.

  **Must NOT do**: Do not touch `lower.rs`/`opt.rs`/`tacky.rs`/`temp.rs` (rewritten in W0-T6 / waves 2+). Do not delete the `src/ir/` directory itself.

  **Recommended Agent Profile**: Category `quick`. File deletion plus small mod.rs cleanup.

  **Parallelization**: Wave 0 Group A (with W0-T3 and W0-T5). Blocked By: W0-T1.

  **References**:
  - Current interpreter to delete: `src/ir/control_flow.rs:1-262`
  - IR module entry to clean: `src/ir/mod.rs:13`
  - OCaml reference has no equivalent (no interpreter); this is pure scaffolding removal.

  **Acceptance Criteria**:
  - [ ] `ls src/ir/control_flow.rs` reports "No such file or directory".
  - [ ] `grep -rn "control_flow" src/` zero matches.
  - [ ] `cargo check --release` zero errors.

  **QA Scenarios**:

  ```
  Scenario: Interpreter file is gone and build still clean
    Tool: Bash
    Steps:
      1. ls src/ir/control_flow.rs (expect "No such file")
      2. grep -rn "control_flow" src/ (expect zero matches)
      3. cargo check --release (expect exit 0)
    Failure Indicators: build failure citing `control_flow::*` -> W0-T1 imports not fully removed
    Evidence: .omo/evidence/task-2-interpreter-deleted.txt
  ```

- [x] 3. W0-T3: Delete the system-C bridge (emit.rs, source.rs, system helpers in toolchain.rs)

  **What to do**:
  - `rm src/codegen/emit.rs` (172-LOC GCC-assembly-text sanitizer).
  - `rm src/support/source.rs` (329-LOC source-text heuristic).
  - In `src/toolchain.rs`: delete or stub `evaluate_with_system_cc`, `system_c_syntax_check`, `system_c_to_assembly`. Keep `preprocess()` (gcc -E -P) and the gcc-final-link helpers intact.
  - In `src/codegen/mod.rs:15-16`: remove `pub use emit::sanitize_system_assembly;` and `pub use lower::emit_native_constant_function;` re-exports.
  - In `src/support/mod.rs`: confirm `mod.rs` no longer re-exports from `source`.

  **Must NOT do**: Do not delete `preprocess()` (gcc -E -P stays) or the gcc final-link helper (native pipeline still uses gcc to assemble+link `.s`).

  **Recommended Agent Profile**: Category `quick`. File deletions plus small re-export trimming.

  **Parallelization**: Wave 0 Group A (with W0-T2). Blocked By: W0-T1.

  **References**:
  - Bridge sanitizer: `src/codegen/emit.rs:1-172` (to delete)
  - Heuristic gate: `src/support/source.rs:1-329` (to delete)
  - Toolchain helpers: `src/toolchain.rs:1-149` (some to delete)
  - Re-exports: `src/codegen/mod.rs:13-16`, `src/support/mod.rs:9`

  **Acceptance Criteria**:
  - [ ] `ls src/codegen/emit.rs src/support/source.rs` both report "No such file or directory".
  - [ ] `grep -rn "sanitize_system_assembly\|compile_with_system_cc_frontend\|SystemAssemblySanitizerOptions" src/` zero matches.
  - [ ] `grep -n "fn preprocess\|fn.*assemble\|fn.*link\|fn.*gcc" src/toolchain.rs` returns >= 1 match (the kept gcc helpers).
  - [ ] `cargo check --release` zero errors.

  **QA Scenarios**:

  ```
  Scenario: System-C bridge fully removed, kept gcc helpers remain
    Tool: Bash
    Steps:
      1. ls src/codegen/emit.rs (expect "No such file")
      2. ls src/support/source.rs (expect "No such file")
      3. grep -n "fn preprocess" src/toolchain.rs (expect >= 1 match)
      4. grep -n "fn.*assemble\|fn.*link\|fn.*gcc" src/toolchain.rs (expect >= 1 match)
      5. cargo check --release (expect exit 0)
    Failure Indicators: gcc helpers missing -> re-add them; sanitizer grep returns hits -> revisit W0-T1
    Evidence: .omo/evidence/task-3-bridge-deleted.txt
  ```

- [x] 4. W0-T4: Create the assembly AST (`src/codegen/assembly.rs`) mirroring `nqcc2/lib/assembly.ml`

  **What to do**:
  - Create `src/codegen/assembly.rs` with enums/structs mirroring `nqcc2/lib/assembly.ml` (129 LOC):
    - `pub enum Reg`: `AX, CX, DX, DI, SI, R8, R9, R10, R11, SP, BP, BX, R12, R13, R14, R15` plus `XMM(i32)` (ch.13 onward).
    - `pub enum Operand`: `Imm(i64), Reg(Reg), Memory(BaseReg, i32), MemoryOffset(...)` and pseudoreg/stack variants.
    - `pub enum Instr`: `Mov, Movsx, MovZeroExtend, Lea, Cmp, BinaryOp, Idiv, Cdq, Call(String), Ret, Push(Operand), Pop(Reg), Jmp(String), JmpCC, SetCC, Label(String), AllocateStack(i32), DeallocateStack(i32), Comment(String)`.
    - `pub enum TopLevel`: `Fn { name, global, instructions }, StaticVariable { name, global, init }, Constant { label, value }`.
    - `pub enum BinaryOpInstr`: `Add, Sub, Mult, DivDouble, DivSigned, RemSigned, BitAnd, BitOr, BitXor, BitShiftLeft, BitShiftRight, AddDouble, SubDouble, MultDouble, DivDoubleDouble, BitShiftLeftDouble`-style per the book.
    - `pub enum ConditionCode`: `E, NE, L, LE, G, GE, A, AE, B, BE, P` (parity for setcc).
    - `pub struct AsmProgram { pub top_level: Vec<TopLevel> }`.
  - Update `src/codegen/mod.rs` to re-export `pub use assembly::{AsmProgram, TopLevel, Instr, Operand, Reg, ConditionCode};`.
  - Add doc comment at top: `// Mirrors nqcc2/lib/assembly.ml (129 LOC). Locked to x86-64 AT&T syntax, System V AMD64 ABI.`

  **Must NOT do**: Do not add platform abstractions or alternate ISAs. Do not add i128 / f32 / f80 / SSE4+ variants. Type system only — no logic.

  **Recommended Agent Profile**: Category `unspecified-high`. Domain modeling of x86-64 assembly requires care; mirror must match OCaml.

  **Parallelization**: Sequential. Blocked By: W0-T1. Blocks: W0-T5+ and any chapter codegen from W2+.

  **References**:
  - OCaml assembly AST: `nqcc2/lib/assembly.ml:1-129`
  - OCaml backend entry: `nqcc2/lib/backend/codegen.ml:4-7`
  - Current 7-line placeholder: `src/codegen/asm.rs:1-7`
  - System V AMD64 reference (cross-check only): https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.99.pdf

  **Acceptance Criteria**:
  - [ ] `grep -n "pub enum Reg\|pub enum Operand\|pub enum Instr\|pub enum TopLevel\|pub enum ConditionCode\|pub struct AsmProgram" src/codegen/assembly.rs` returns one match per enum.
  - [ ] `cargo check --release` zero errors.
  - [ ] `wc -l src/codegen/assembly.rs` >= 80.
  - [ ] `grep -n "nqcc2/lib/assembly.ml" src/codegen/assembly.rs` >= 1 match.

  **QA Scenarios**:

  ```
  Scenario: Assembly AST enums compile and are reachable
    Tool: Bash (cargo)
    Steps:
      1. Write temporary `src/bin/check_asm.rs` that constructs `AsmProgram { top_level: vec![TopLevel::Fn { name: "main".into(), global: true, instructions: vec![Instr::Mov { src: Operand::Imm(2), dst: Operand::Reg(Reg::AX) }, Instr::Ret] }] }`.
      2. cargo check --bin check_asm (expect exit 0)
      3. rm src/bin/check_asm.rs
      4. cargo check --release (expect exit 0)
    Failure Indicators: error mentions any new enum -> fix variant name; warning dead_code -> acceptable
    Evidence: .omo/evidence/task-4-asm-ast-builds.txt
  ```

- [x] 5. W0-T5: Restructure `src/codegen/` to mirror `nqcc2/lib/backend/` (stub module skeletons)

  **What to do**:
  - Create stubs under `src/codegen/`:
    - `src/codegen/codegen.rs`: `pub fn generate(tacky: &TackyProgram, frames: &[Frame]) -> Result<AsmProgram>` body `unimplemented!("ch.1+ codegen wired in wave 2+")`; comment `// Mirrors nqcc2/lib/backend/codegen.ml`.
    - `src/codegen/fixup.rs`: `pub fn fixup(asm: AsmProgram, frames: &[Frame]) -> Result<AsmProgram>` body `unimplemented!("ch.9+ fixup wired in wave 10")`; comment `// Mirrors nqcc2/lib/backend/instruction_fixup.ml`.
    - `src/codegen/replace_pseudos.rs`: stub `pub fn replace_pseudos(...)`; comment `// Mirrors nqcc2/lib/backend/replace_pseudos.ml`.
    - `src/codegen/regalloc/mod.rs`: stub `pub fn allocate(asm: AsmProgram) -> Result<AsmProgram>`; comment `// Mirrors nqcc2/lib/backend/regalloc.ml (651 LOC; uses Briggs/George coalescing)`.
    - `src/codegen/assembly_symbols.rs`: stub `pub struct AsmSymbols {}` with `Default`; comment `// Mirrors nqcc2/lib/backend/assembly_symbols.ml`.
    - `src/codegen/emit.rs`: stub `pub fn emit(program: &AsmProgram) -> Result<String>`; comment `// Mirrors nqcc2/lib/emit.ml (Format-based pretty-printing)`.
    - `src/codegen/abi.rs`: stub `pub enum ParamClass { Int, SSE, Memory }` plus `pub struct AbiPlan {}`; comment `// Mirrors nqcc2/lib/backend/codegen.ml ABI section`.
    - `src/codegen/frame.rs`: stub `pub struct FrameLayout {}`; comment `// Mirrors nqcc2 stack-frame knowledge, scattered across codegen.ml/assembly_symbols.ml`.
  - Delete `src/codegen/lower.rs` (its `emit_native_constant_function` is dead with the bridge gone).
  - Delete `src/codegen/register_allocator.rs` (subsumed into `regalloc/mod.rs`).
  - Update `src/codegen/mod.rs` to declare all real modules plus re-exports for `generate`, `fixup`, `replace_pseudos`, `allocate`, `emit`.

  **Must NOT do**: Do not implement logic. Do not duplicate `asm.rs`. Do not touch W0-T4's `assembly.rs` content.

  **Recommended Agent Profile**: Category `unspecified-high`. Module restructuring across many files.

  **Parallelization**: Wave 0 Group B (with W0-T6). Blocked By: W0-T1, W0-T4. Blocks: chapter codegen tasks from W2+.

  **References**:
  - OCaml backend dir: `nqcc2/lib/backend/`
  - Old placeholders: `src/codegen/{abi,asm,frame,register_allocator}.rs`
  - Module entry: `src/codegen/mod.rs:1-16`

  **Acceptance Criteria**:
  - [ ] `ls src/codegen/` shows: `assembly.rs assembly_symbols.rs abi.rs codegen.rs emit.rs fixup.rs frame.rs mod.rs regalloc/ replace_pseudos.rs` (no `asm.rs`, `lower.rs`, `register_allocator.rs`).
  - [ ] `grep -rn "unimplemented!" src/codegen/` returns >= 5 matches.
  - [ ] `cargo check --release` zero errors.
  - [ ] `wc -l src/codegen/mod.rs` >= 15.

  **QA Scenarios**:

  ```
  Scenario: Codegen module structure matches nqcc2/lib/backend/
    Tool: Bash
    Steps:
      1. ls src/codegen/ (expect all 10 expected entries including regalloc/)
      2. grep -rn "unimplemented!" src/codegen/ (expect >= 5)
      3. cargo check --release (expect exit 0)
    Failure Indicators: missing file -> add the stub; compile error -> fix mod.rs declarations
    Evidence: .omo/evidence/task-5-codegen-scaffold.txt
  ```

- [x] 6. W0-T6: Restructure `src/semantics/` into three passes + restructure `src/ir/` for book-faithful TACKY

  **What to do**:
  - Replace placeholder files in `src/semantics/`:
    - `src/semantics/resolve.rs`: `pub fn resolve_program(ast: &Program) -> Result<ResolvedProgram>` body `unimplemented!("resolve wired in wave 6")`; comment `// Mirrors nqcc2/lib/semantic_analysis/resolve.ml`.
    - `src/semantics/label_loops.rs`: `pub fn label_loops(ast: &mut Program)` body `unimplemented!()`; comment `// Mirrors nqcc2/lib/semantic_analysis/label_loops.ml`.
    - `src/semantics/typecheck.rs`: `pub fn typecheck(ast: &ResolvedProgram) -> Result<TypedProgram>` body `unimplemented!("typecheck wired in wave 12+")`; comment `// Mirrors nqcc2/lib/semantic_analysis/typecheck.ml`.
    - DELETE `src/semantics/validate.rs` (its scope/label/goto logic moves into `label_loops.rs` and `typecheck.rs`).
    - DELETE `src/semantics/{names,symbols,types,layout}.rs`.
  - Update `src/semantics/mod.rs` to declare the three real modules plus re-exports.
  - Restructure `src/ir/`:
    - Replace `src/ir/tacky.rs` 31-LOC stub with enums mirroring `nqcc2/lib/tacky.ml`: `pub enum Instruction` (Return, SignExtend, ZeroExtend, Truncate, IntToDouble, DoubleToInt, UIntToDouble, DoubleToUInt, Add, Sub, Mul, DivSigned, RemSigned, BitAnd, BitOr, BitXor, BitShiftLeft, BitShiftRight, Negate, Complement, Not, Jump, JumpIfZero, JumpIfNotZero, Label, Copy, Load, Store, GetAddress, AddPtr, Call). Plus `pub enum Val { Constant(i64), Var(String) /* ch.13: ConstantDouble(f64) */ }`. Plus `pub struct TackyProgram { pub functions: Vec<TackyFunction> }`. Stub `pub fn ast_to_tacky(...)` body `unimplemented!()`.
    - Replace `src/ir/opt.rs` 12-LOC placeholder with: `pub enum OptPass { ConstantFolding, UnreachableCodeElim, CopyPropagation, DeadStoreElim }` plus `pub fn run_opt(...)` body `unimplemented!()`. Mirror `nqcc2/lib/optimizations/optimize.ml`.
    - Replace `src/ir/temp.rs` 7-LOC placeholder with `pub struct TempId(u32); pub struct TempIdGenerator(u32);` impl `next()`. Mirror OCaml `unique_ids.ml`.
    - Create `src/ir/cfg.rs` stub: `pub struct Cfg<N: AsRef<Instruction>> {}` plus `pub fn build<N>(program: &N) -> Cfg<N>` `unimplemented!()`. Keep it minimal; wave 20 fills in the trait bound.
  - Update `src/ir/mod.rs` to declare: `pub mod tacky; pub mod lower; pub mod opt; pub mod cfg; pub mod temp;`. Remove all references to `control_flow`.

  **Must NOT do**: Do not implement resolve/label_loops/typecheck logic (stubs only). Do not change `src/parse/`, `src/lex/`, or `src/ast/`. Do not add visitor patterns or generic AST walkers.

  **Recommended Agent Profile**: Category `unspecified-high`. Semantic-passes scaffold + TACKY IR mirroring.

  **Parallelization**: Wave 0 Group B (with W0-T5). Blocked By: W0-T1. Blocks: W0-T7.

  **References**:
  - OCaml semantic analysis dir: `nqcc2/lib/semantic_analysis/`
  - OCaml TACKY: `nqcc2/lib/tacky.ml:1-99`, `nqcc2/lib/tacky_gen.ml:1-593`
  - OCaml optimizations dir: `nqcc2/lib/optimizations/`
  - Existing TACKY: `src/ir/tacky.rs:1-31`
  - Existing semantics placeholders: `src/semantics/{mod,validate,names,symbols,types,layout}.rs`

  **Acceptance Criteria**:
  - [ ] `ls src/semantics/` shows: `mod.rs resolve.rs label_loops.rs typecheck.rs` (no `validate.rs`, no other files).
  - [ ] `ls src/ir/` shows: `mod.rs tacky.rs lower.rs opt.rs cfg.rs temp.rs`.
  - [ ] `grep -rn "control_flow" src/` zero matches.
  - [ ] `cargo check --release` zero errors.

  **QA Scenarios**:

  ```
  Scenario: Semantics restructured into 3 passes; TACKY IR reflects OCaml
    Tool: Bash
    Steps:
      1. ls src/semantics/ src/ir/ (expect canonical file lists)
      2. grep -rn "control_flow" src/ (expect zero matches)
      3. cargo check --release (expect exit 0)
    Failure Indicators: missing module -> add the stub; compile error in mod.rs -> fix declarations
    Evidence: .omo/evidence/task-6-semantics-ir-scaffold.txt
  ```

- [x] 7. W0-T7: Verify the foundation rewrite is clean (zero warnings, no stale fingerprints, all unit tests pass)

  **What to do**: After W0-T1..T6 are complete, run a comprehensive grep + build + test sweep. Confirm:
  - `cargo build --release` reports zero warnings, exit 0.
  - `cargo test --release` runs the 9 pre-existing unit tests; all pass.
  - `grep -rn "evaluate_program\|should_defer_parse_to_system_frontend\|sanitize_system_assembly\|compile_with_system_cc_frontend\|emit_native_constant_function\|control_flow::evaluate" src/` returns zero matches.
  - `ls src/ir/control_flow.rs src/codegen/emit.rs src/support/source.rs` all return "No such file or directory".
  - `tree src/` output (or `find src/ -name "*.rs"`) shows the OCaml-mirror structure: `src/semantics/{resolve,label_loops,typecheck}.rs`, `src/codegen/{assembly,assembly_symbols,abi,codegen,emit,fixup,frame,regalloc,replace_pseudos}.rs`, `src/ir/{tacky,lower,opt,cfg,temp}.rs`.
  - Update `docs/COACHING_LOG.md` with a "Wave 0 Complete" section recording the actual gate commands run.

  **Must NOT do**: Do not modify code in this task; this is a verification + log update only.

  **Recommended Agent Profile**: Category `quick`. Grep + build verification.

  **Parallelization**: Sequential gate. Blocked By: W0-T1..T6. Blocks: Wave 1+.

  **References**: All Wave-0 tasks; `docs/COACHING_LOG.md`.

  **Acceptance Criteria**:
  - [ ] `cargo build --release` zero warnings, exit 0 (record stdout to evidence file).
  - [ ] `cargo test --release` shows 9 passed, 0 failed.
  - [ ] All 4 grep checks return zero matches.
  - [ ] `find src/ -name "*.rs"` shows the OCaml-mirror file layout.
  - [ ] `docs/COACHING_LOG.md` has a Wave 0 verification section.

  **QA Scenarios**:

  ```
  Scenario: Foundation rewrite gate - build, test, and fingerprint cleanup
    Tool: Bash
    Steps:
      1. cargo build --release 2>&1 | tee /tmp/w0-build.log
         - expect: "Finished release", exit 0, no warnings
      2. cargo test --release 2>&1 | tee /tmp/w0-test.log
         - expect: "test result: ok. 9 passed; 0 failed"
      3. grep -rn "evaluate_program\|should_defer_parse_to_system_frontend\|sanitize_system_assembly\|compile_with_system_cc_frontend\|emit_native_constant_function\|control_flow::evaluate" src/
         - expect: zero matches
      4. ls src/ir/control_flow.rs src/codegen/emit.rs src/support/source.rs
         - expect: all "No such file or directory"
    Failure Indicators: any non-zero result -> re-run offending wave task
    Evidence: .omo/evidence/task-7-wave0-gate.txt
  ```

- [x] 8. W1-T1: Port `nqcc2/lib/lex.ml` + `nqcc2/lib/tokens.ml` to `src/lex/`

  **What to do**:
  - Open `nqcc2/lib/tokens.ml` (lexer token types) and mirror exactly into `src/lex/token.rs`. Replace the existing 88-LOC `src/lex/token.rs` with the book-faithful types: `pub enum Token { Identifier(String), IntConstant(i64), LongConstant(i64), UIntConstant(u64), ULongConstant(u64), DoubleConstant(f64), CharConstant(i64), StringConstant(String), Keyword(Keyword), Punct(Punct) }` (the book introduces new tokens chapter by chapter; in this port, define all tokens up front since the test suite asserts on every token at the right chapter; see `nqcc2/lib/tokens.ml` for the canonical list).
  - Replace `src/lex/scanner.rs` 551-LOC existing scanner with a port of `nqcc2/lib/lex.ml` (211 LOC). Preserve existing correct behavior: line/column comments, char escapes (ch.16), string-literal handling (ch.16), long-literal parsing (ch.11), unsigned-literal parsing (ch.12), float-literal parsing (ch.13), identifier-vs-keyword distinction.
  - Delete the `src/lex/cursor.rs` placeholder (book uses stream-style; the Rust port does not need a cursor abstraction — the scanner is a character iterator).
  - Keep `src/lex/keyword.rs` updated; the existing 34-LOC table needs to grow to include every book keyword (`int`, `long`, `unsigned`, `double`, `char`, `void`, `if`, `else`, `do`, `while`, `for`, `break`, `continue`, `return`, `goto`, `switch`, `case`, `default`, `sizeof`, `struct`, `union`, `static`, `extern`).
  - Update `src/lex/mod.rs` to re-export `pub use token::{Token, Keyword, Punct, Constant};` and `pub use scanner::scan;` plus `pub use scanner::pretty_tokens;` (used by `--stage lex`).

  **Must NOT do**: Do not implement regex-based lexing (book uses character-by-character). Do not implement stream-style buffering (Rust has `Chars`). Do not add features outside the book.

  **Recommended Agent Profile**: Category `unspecified-high`. Lexer correctness is critical; the OCaml uses regex for identifiers and the Rust port should mirror that via `regex` crate or hand-rolled.

  **Parallelization**: Sequential blocker for chapters 1+. Blocked By: W0-T7. Blocks: W2+.

  **References** (mirror exactly):
  - OCaml token types: `nqcc2/lib/tokens.ml:1-100`
  - OCaml lexer: `nqcc2/lib/lex.ml:1-211`
  - Existing scanner to replace: `src/lex/scanner.rs:1-551`
  - Existing token: `src/lex/token.rs:1-88`
  - Existing keyword: `src/lex/keyword.rs:1-34`
  - Existing cursor placeholder: `src/lex/cursor.rs:1-14` (to delete)

  **Acceptance Criteria**:
  - [ ] `cargo check --release` zero errors.
  - [ ] For sample `int main(void) { return 2; }`, `cargo run --release -- --stage lex path/to/file.c` matches expected token sequence per `nqcc2/test/test_lex.ml`.
  - [ ] `grep -c "fn scan" src/lex/scanner.rs` >= 1.
  - [ ] `grep -rn "cursor.rs\|LexCursor" src/lex/` zero matches.

  **QA Scenarios**:

  ```
  Scenario: Lexer port round-trips the chapter 1 token sample
    Tool: Bash (cargo)
    Steps:
      1. Build: cargo build --release (expect exit 0)
      2. Write test input `echo 'int main(void) { return 2; }' > /tmp/ch1.c`
      3. Run `target/release/rustcc /tmp/ch1.c --stage lex` (expect tokens for: int, main, (, void, ), {, return, 2, ;, }, EOF)
      4. Compare against expected token sequence printed by `cd nqcc2 && dune exec nqcc2 -- /tmp/ch1.c 2>&1 | head -20` (cross-check; the OCaml must be installed for this)
    Failure Indicators: missing tokens (e.g., no `IntConstant(2)`) -> extend the scanner; wrong keywords -> fix keyword table
    Evidence: .omo/evidence/task-8-lexer-port.txt
  ```

- [x] 9. W2-T1: Chapter 1 - AST node + parser + single-function Program

  **What to do**:
  - In `src/ast/`, define the AST node set for chapter 1: `pub enum Expr { Constant(i64) }`, `pub enum Stmt { Return(Expr) }`, `pub struct Function { pub name: String, pub body: Vec<Stmt> }`, `pub struct Program { pub function: Function }`. Mirror `nqcc2/lib/ast.ml` (chapter 1's subset of variants). Place each enum/struct in its own file: `src/ast/{expr,stmt,item,decl,operator,ty}.rs`. The existing `src/ast/` skeleton mostly fits; extend the enums.
  - Replace `src/parse/parser.rs` with a port of `nqcc2/lib/parse.ml` restricted to chapter 1 grammar:
    ```
    program     := int identifier '(' void ')' '{' 'return' int_constant ';' '}'
    ```
    Implement recursive-descent: `parse_program`, `parse_function`, `parse_statement`. Each consumes tokens with `self.peek()`, `self.advance()`, `self.expect(tok_kind)`, returning `Result<T>` with `CompileError`.
  - Delete `src/parse/cursor.rs` placeholder; the parser's character-level iteration is now token-level.
  - Keep `src/parse/precedence.rs` (will be expanded in W3+).
  - Update `src/parse/mod.rs` to `pub mod parser; pub mod precedence; pub use parser::parse_program;`.
  - For chapter 1, the semantic-analysis passes resolve/label_loops/typecheck can be stubs (already are from W0) — call them after parse, but they pass through unchanged for single-function int-returning programs.

  **Must NOT do**: Do not yet support `void` parameter lists beyond chapter 1. Do not add any operator other than chapter 1 needs.

  **Recommended Agent Profile**: Category `unspecified-high`. Recursive-descent parser for a constrained grammar; AST node modeling.

  **Parallelization**: Sequential blocker for chapter 1 gate. Blocked By: W1-T1.

  **References**:
  - OCaml AST: `nqcc2/lib/ast.ml:1-206` (chapter 1's subset is just `Program`, `Function`, `Return`, `Constant(i64)`)
  - OCaml parser: `nqcc2/lib/parse.ml:1-973` (chapter 1 = first ~80 lines)
  - Existing AST skeleton: `src/ast/{mod,expr,stmt,item,decl,operator,ty}.rs`
  - Existing parser: `src/parse/parser.rs:1-431` (chapter 1 grammar subset)
  - Existing cursor placeholder: `src/parse/cursor.rs:1-14` (to delete)
  - Book chapter guide: `docs/book/ch01-minimal-compiler.md`
  - Stage pseudocode: `docs/stages/ch01-minimal-compiler.md`

  **Acceptance Criteria**:
  - [ ] `cargo check --release` zero errors.
  - [ ] `target/release/rustcc /tmp/ch1.c --stage parse` produces a non-empty AST tree dump.
  - [ ] Parsing `int main(void) { return 2; }` succeeds; parsing `int main(void) { return ; }` returns an error.

  **QA Scenarios**:

  ```
  Scenario: Chapter 1 parser produces AST
    Tool: Bash (cargo)
    Steps:
      1. cargo build --release (expect exit 0)
      2. target/release/rustcc /tmp/ch1.c --stage parse (expect non-empty AST representation)
      3. target/release/rustcc /tmp/bad.c --stage parse where bad.c = "int main(void) { return ; }" (expect non-zero exit, stderr message)
    Failure Indicators: missing variants -> add to AST/parser; token consumed out of order -> fix parser
    Evidence: .omo/evidence/task-9-ch1-parser.txt
  ```

- [x] 10. W2-T2: Chapter 1 - TACKY IR (full enum surface as defined in W0-T6) + AST->TACKY for ch.1

  **What to do**:
  - Confirm TACKY enums from W0-T6 are sufficient for chapter 1: `TackyProgram { functions: Vec<TackyFunction> }`, `TackyFunction { name: String, body: Vec<Instruction> }`, `Instruction::Return(Val::Constant(2))`. No new TACKY variants needed for chapter 1.
  - Implement `pub fn ast_to_tacky(ast: &Program) -> Result<TackyProgram>` in `src/ir/lower.rs`. Chapter 1 version handles `Program { function: Function { name, body } }` mapping to `TackyFunction { name, body: [Instruction::Return(Val::Constant(N))] }`.
  - The function should require `&TypedProgram` input from typecheck (but for ch.1, typecheck is a no-op pass-through). To bridge, define `pub type TypedProgram = Program;` (a type alias) in `src/ir/lower.rs` for now. Chapter 12+ will replace this with the real TypedProgram and the bridge becomes a real conversion.

  **Must NOT do**: Do not introduce a real TypedProgram type. Do not extend TACKY.

  **Recommended Agent Profile**: Category `unspecified-high`. Mechanical AST->TACKY translation.

  **Parallelization**: Parallel with W2-T3. Blocked By: W2-T1.

  **References**:
  - OCaml TACKY gen: `nqcc2/lib/tacky_gen.ml:1-593` (chapter 1's relevant code is a few lines)
  - OCaml TACKY: `nqcc2/lib/tacky.ml:1-99`
  - IR lowerer: `src/ir/lower.rs:1-276` (replace with book-faithful version; current 276-LOC interpreter is unrelated)
  - AST: `src/ast/` (chapter 1 subset from W2-T1)

  **Acceptance Criteria**:
  - [ ] `cargo check --release` zero errors.
  - [ ] `cargo run --release -- /tmp/ch1.c --stage tacky` outputs a TACKY program with one function and one `Return(2)`.

  **QA Scenarios**:

  ```
  Scenario: Chapter 1 AST->TACKY lowers the return-int program
    Tool: Bash (cargo)
    Steps:
      1. cargo build --release
      2. target/release/rustcc /tmp/ch1.c --stage tacky (expect TACKY dump with one fn returning 2)
    Failure Indicators: missing TACKY variants -> already defined in W0-T6; wrong variant -> recheck TACKY enum
    Evidence: .omo/evidence/task-10-ch1-tacky.txt
  ```

- [x] 11. W2-T3: Chapter 1 - assembly emission (`generate` + `fixup_noop` + `replace_pseudos_noop` + `emit`) for ch.1

  **What to do**:
  - Implement `src/codegen/codegen.rs::generate(...)` for chapter 1: takes `&TackyProgram` and produces an `AsmProgram` with one `TopLevel::Fn { name: "main", global: true, instructions: [Mov { src: Operand::Imm(2), dst: Operand::Reg(Reg::AX) }, Ret] }`.
  - Implement `src/codegen/fixup.rs::fixup(...)` as a chapter-1 no-op: returns the input unchanged. Add comment `// ch.1 has no fixups; this is identity. Real fixups land in W10 (ch.9+).`
  - Implement `src/codegen/replace_pseudos.rs::replace_pseudos(...)` as a chapter-1 no-op: returns input unchanged. Add comment `// ch.1 has no pseudoregisters; real replace_pseudos lands in W21 (ch.20).`
  - Implement `src/codegen/emit.rs::emit(...)`: format `AsmProgram` as AT&T-syntax `.s` text. Use `std::fmt::Write`. Output should look like:
    ```
    .globl main
    main:
        movl $2, %eax
        ret
    ```
  - Wire the pipeline in `src/compiler.rs::compile()`: lex -> parse -> resolve(no-op) -> label_loops(no-op) -> typecheck(no-op) -> ast_to_tacky -> tacky_opt(no-op) -> tacky_to_asm(=codegen::generate) -> fixup -> replace_pseudos -> emit.

  **Must NOT do**: Do not implement pseudo->real-register replacement (w21). Do not implement real instructions beyond `Mov`/`Ret`/`Cmp`/`BinaryOp` (we add them as needed in W3+). Do not use MASM/Intel syntax.

  **Recommended Agent Profile**: Category `deep`. Assembly emission is critical and easy to get subtly wrong.

  **Parallelization**: Parallel with W2-T2. Blocked By: W2-T1.

  **References**:
  - OCaml codegen: `nqcc2/lib/backend/codegen.ml:1-919` (chapter 1 = the simplest few lines)
  - OCaml emit: `nqcc2/lib/emit.ml:1-349` (ch.1 = simple format)
  - OCaml instruction fixup: `nqcc2/lib/backend/instruction_fixup.ml:1-251`
  - OCaml replace pseudos: `nqcc2/lib/backend/replace_pseudos.ml:1-137`
  - Assembly AST: `src/codegen/assembly.rs` (W0-T4)
  - Codegen stub to fill: `src/codegen/codegen.rs`

  **Acceptance Criteria**:
  - [ ] For `int main(void) { return 2; }`, `target/release/rustcc /tmp/ch1.c` produces an executable that exits with code 2.
  - [ ] `--stage codegen` outputs valid AT&T-syntax assembly.
  - [ ] `--stage run` runs the produced executable.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 1 --latest-only` reports every test passing (specifically `chapter_1/valid/*` and rejects `chapter_1/invalid_lex/*` + `chapter_1/invalid_parse/*`).

  **QA Scenarios**:

  ```
  Scenario: Chapter 1 end-to-end compile+run
    Tool: Bash (gcc + ./target/release/rustcc + execute)
    Steps:
      1. cargo build --release
      2. target/release/rustcc /tmp/ch1.c (expect exit 0; produces /tmp/ch1 executable or main.o)
      3. /tmp/ch1 (expect exit code 2)
      4. echo $? (expect 2)
    Failure Indicators: exit code != 2 -> check assembly; gcc error -> check labels/registers
    Evidence: .omo/evidence/task-11-ch1-end-to-end.txt

  Scenario: Chapter 1 official test gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 1 --latest-only --expected-error-codes 1 2
      3. Save full output to .omo/evidence/task-11-chapter-gate.txt
      4. Confirm exit code 0 and zero 'fail' lines
    Failure Indicators: any `fail` line -> debug before marking complete
    Evidence: .omo/evidence/task-11-chapter-gate.txt
  ```

- [x] 12. W3-T1: Chapter 2 - unary `-` and `~` operators (parser, AST, TACKY, codegen)

  **What to do**: Add unary operators to the AST, parser, and TACKY lowering.
  - Extend `src/ast/expr.rs`: add `UnaryOp` enum (`Negate`, `Complement`) and `Expr::Unary { op: UnaryOp, expr: Box<Expr> }` variant. Add unary-related fields to `src/ast/operator.rs` if needed.
  - Extend `src/parse/parser.rs` `parse_factor`: when token is `-`, parse `<factor>` then build `Unary { Negate, ... }`; same for `~` -> `Complement`.
  - Extend `src/ir/lower.rs::ast_to_tacky`: lower `Unary::Negate` to TACKY `Instruction::Negate { dst: tmp_var }` then `Copy { src: tmp_var, dst: dst_var }`. Same for `Complement` -> `Instruction::Complement`.
  - Extend `src/codegen/codegen.rs::generate`: lower TACKY `Negate` and `Complement` to assembly. `Negate` => `movl src, dst; negl dst`. `Complement` => `movl src, dst; notl dst`. Use the same `tmp_var` allocation pattern as `parse.ml`'s chapter 2 section.

  **Must NOT do**: Do not add `&` (address-of) - that's chapter 14. Do not add `*` deref - chapter 14. Do not add `!` logical not - chapter 4.

  **Recommended Agent Profile**: Category `unspecified-high`. Mechanism for unary lowering is straightforward but needs to be done right.

  **Parallelization**: Parallel with W3-T2. Blocked By: W2-T3 (chapter 1 gate green).

  **References**:
  - OCaml parse.ml chapter 2 ~lines 80-150
  - OCaml tacky_gen.ml chapter 2 ~lines 30-80
  - OCaml codegen.ml chapter 2 ~lines 30-100 (`Negate`, `Complement` codegen)
  - Book: `docs/book/ch02-unary-operators.md`, `docs/stages/ch02-unary-operators.md`

  **Acceptance Criteria**:
  - [ ] For `int main(void) { return -3; }`, `target/release/rustcc /tmp/ch2_neg.c && /tmp/ch2_neg` exits with code `4294967293` (or whatever the correct post-`neg` value is per `gcc`'s handling; cross-check with `nqcc2`).
  - [ ] For `int main(void) { return ~0; }`, exit code is `255`.
  - [ ] For `int main(void) { return -(-3); }`, exit code is 3.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 2 --latest-only` reports every test passing.

  **QA Scenarios**:

  ```
  Scenario: Chapter 2 unary gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 2 --latest-only
      3. Save output to .omo/evidence/task-12-chapter-gate.txt; assert zero `fail` lines and exit 0
    Failure Indicators: any `fail` -> debug unary lowering
    Evidence: .omo/evidence/task-12-chapter-gate.txt
  ```

- [x] 13. W3-T2: Chapter 2 - chapter gate verification + COACHING_LOG update

  **What to do**: Run `./tests/test_compiler ./target/release/rustcc --chapter 2 --latest-only` and update `docs/COACHING_LOG.md` with the wave-3 chapter gate having been actually run. Capture stdout+stderr to `.omo/evidence/`.

  **Must NOT do**: Do not modify compiler code in this task.

  **Recommended Agent Profile**: Category `quick`. Test invocation + log update.

  **Parallelization**: Parallel with W3-T1 (logs after the gate is conceptually ready).

  **References**: `docs/COACHING_LOG.md`, `tests/test_compiler`, `docs/book/test-map.md` row for chapter 2.

  **Acceptance Criteria**:
  - [ ] `/tmp/ch2_neg.c` -> exit-code-3-style run cross-checked against nqcc2's output for the same file.
  - [ ] `docs/COACHING_LOG.md` has a Wave 3 (Chapter 2) entry with the actual command run and result.

  **QA Scenarios**:

  ```
  Scenario: Cross-check chapter 2 outputs against nqcc2 reference
    Tool: Bash
    Steps:
      1. cargo build --release
      2. For each test in tests/tests/chapter_2/valid/, compile with rustcc and with nqcc2 (if available); both should produce executables that exit with the same code
      3. If any divergence -> fix the corresponding unary codegen
    Failure Indicators: exit-code mismatch -> debug
    Evidence: .omo/evidence/task-13-ch2-cross-check.txt
  ```

- [x] 14. W4-T1: Chapter 3 - binary operators `+ - * / %` + parser precedence + bitwise extras `& | ^ << >>`

  **What to do**: Add the binary operator family to AST, parser (with precedence), TACKY, codegen. The bitwise extras `& | ^ << >>` also land here.
  - Extend `src/ast/operator.rs`: add `pub enum BinaryOp { Add, Sub, Mul, DivSigned, RemSigned, BitAnd, BitOr, BitXor, BitShiftLeft, BitShiftRight, ...}`. Order matters for precedence parsing.
  - Extend `src/parse/precedence.rs`: define `pub enum Precedence { ..., AddSub, MulDiv, BitShift, BitAnd, BitXor, BitOr }`. C operator precedence: `* / %` > `+ -` > `<< >>` > `< > <= >=` (ch.4) > `== !=` (ch.4) > `&` > `^` > `|` > `&&` `||` (ch.4).
  - Implement `src/parse/parser.rs::parse_binary_expr(precedence)` using precedence climbing: function-recursive entry point, climbing while the next token's precedence > current. Each level calls into the next-higher precedence.
  - Extend `src/ir/tacky.rs::Instruction` with the missing binary variants: `Mul, DivSigned, RemSigned, BitAnd, BitOr, BitXor, BitShiftLeft, BitShiftRight`.
  - Extend `src/ir/lower.rs::ast_to_tacky`: for each `Expr::Binary { op, left, right }`, allocate a fresh `tmp_var`, lower left/right to TACKY `Val`s, emit the corresponding binary `Instruction`.
  - Extend `src/codegen/codegen.rs::generate` to lower each binary TACKY variant: `Add` => `addl src, dst`, `Sub` => `subl`, `Mul` => `imull`, `DivSigned` => `movl left, %eax; cdq; idivl right; movl %eax, dst`, `RemSigned` => same but `%edx`, `BitAnd` => `andl`, `BitOr` => `orl`, `BitXor` => `xorl`, `BitShiftLeft` => `movl left, %ecx; sall %cl, dst` (use `cl` for shift count per book).
  - Add `pub use src/parse/precedence::Precedence` to `src/parse/mod.rs`.

  **Must NOT do**: Do not add the relational/logical operators (`< > == != && ||`) - chapter 4. Do not add unsigned variants - chapter 12. Do not add float arithmetic - chapter 13.

  **Recommended Agent Profile**: Category `deep`. Precedence-climbing parser is well-trodden but error-prone; bitwise codegen has subtle register-allocation concerns (`cl` for shifts).

  **Parallelization**: Parallel with W4-T2 (which is the bitwise extra codegen verification). Blocked By: W3-T2.

  **References**:
  - OCaml `parse.ml:1-973` (chapters 2-3 cover expressions)
  - OCaml `tacky.ml:1-99` and `tacky_gen.ml:1-593` (binary variants)
  - OCaml `codegen.ml:1-919` (`Add`, `Sub`, `Mul`, `Div`, `BitAnd`, etc.)
  - Book: `docs/book/ch03-binary-operators.md`, `docs/stages/ch03-binary-operators.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { return 1 + 2 * 3; }` exits with 7.
  - [ ] `int main(void) { return 12 % 5; }` exits with 2.
  - [ ] `int main(void) { return (1 << 3) | (2 & 0xf0); }` exits with `8`.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise` reports every test passing.

  **QA Scenarios**:

  ```
  Scenario: Chapter 3 + bitwise extras gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise
      3. Save output to .omo/evidence/task-14-chapter-gate.txt; assert zero `fail` lines, exit 0
    Failure Indicators: any `fail` -> precedence or codegen bug
    Evidence: .omo/evidence/task-14-chapter-gate.txt
  ```

- [x] 15. W4-T2: Chapter 3 - codegen landing (binary ops + bitwise + double-check precedence)

  **What to do**: Run the codegen for a binary-heavy C program (e.g., a calculator-like snippet from `tests/tests/chapter_3/valid/bitwise/`) and confirm the output assembly uses `imull`, `idivl`, `andl`, `orl`, `xorl`, `sall` correctly. Update the `--chapter 3` gate by re-running. Capture evidence.

  **Recommended Agent Profile**: Category `unspecified-high`. Codegen spot-check + gate re-run.

  **Parallelization**: Parallel with W4-T1.

  **References**: Same as W4-T1; `tests/tests/chapter_3/valid/bitwise/`.

  **Acceptance Criteria**: Same as W4-T1 plus a cross-check of `target/release/rustcc input.c --stage codegen` output for sample bitwise programs.

  **QA Scenarios**:

  ```
  Scenario: Chapter 3 bitwise extra cross-check via assembly inspection
    Tool: Bash
    Steps:
      1. cargo build --release
      2. For a sample bitwise program from tests/tests/chapter_3/valid/bitwise/, run `target/release/rustcc sample.c --stage codegen` and confirm assembly contains the expected instructions (andl, orl, xorl, sall)
      3. Compare against `nqcc2`'s output if available
    Failure Indicators: missing instructions -> fix codegen
    Evidence: .omo/evidence/task-15-bitwise-codegen-check.txt
  ```

- [x] 16. W4-T3: Chapter 3 - gate rerun + COACHING_LOG update + commit

  **What to do**: Run `./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise` once more, update COACHING_LOG, commit. Same protocol as W3-T2.

  **Recommended Agent Profile**: Category `quick`. Verification + log + commit.

  **Parallelization**: Sequential; final step of Wave 4.

  **References**: Same as W3-T2.

  **Acceptance Criteria**: gate green; COACHING_LOG updated; chapter-3 commit made.

  **QA Scenarios**:

  ```
  Scenario: Chapter 3 final verification + commit
    Tool: Bash (git + Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise (expect green)
      3. Update COACHING_LOG.md; commit
    Failure Indicators: gate red -> re-debug
    Evidence: .omo/evidence/task-16-ch3-final.txt
  ```

- [x] 17. W5-T1: Chapter 4 - logical (`&& || !`) and relational (`== != < > <= >=`) operators

  **What to do**: Add `||`, `&&`, `!`, `==`, `!=`, `<`, `>`, `<=`, `>=` to AST/parser/TACKY/codegen. Precedence: relational > equality > `&&` > `||`. Logical ops short-circuit (jump over right side).
  - Extend `src/ast/operator.rs` with relational/logical `BinaryOp` variants. Extend `UnaryOp` with `Not`.
  - Extend `src/parse/precedence.rs` with `Relational, Equality, LogicalAnd, LogicalOr` precedence levels. Lower numbers = higher binding power per the book.
  - Implement short-circuit lowering in `src/ir/lower.rs::ast_to_tacky`: `e1 && e2` => evaluate `e1` into a tmp; if zero, jump to label `false_label`; evaluate `e2`; jump `false_label` if zero; set result to 1 else 0. `||` symmetric. Book-correct implementation per `nqcc2/lib/tacky_gen.ml` `LogicalAnd`/`LogicalOr`.
  - Add TACKY variants for boolean normalization (book uses `SignExtend` to convert int<->bool) and `JumpIfZero`/`JumpIfNotZero` to labels (already present from W0-T6).
  - Codegen for `==`, `!=`, `<`, `<=`, `>`, `>=` into `cmpl`, then `setCC` (`sete`, `setne`, `setl`, `setle`, `setg`, `setge`) per OCaml `codegen.ml` chapter 4.
  - Update `--chapter 4 --latest-only --bitwise` gate.

  **Must NOT do**: Do not yet add float comparisons (`==` on `double`) - chapter 13 with NaN.

  **Recommended Agent Profile**: Category `deep`. Short-circuit lowering and boolean normalization are non-trivial.

  **Parallelization**: Sequential after W4-T3.

  **References**:
  - OCaml `parse.ml` chapter 4 ~lines 150-280
  - OCaml `tacky_gen.ml` `LogicalAnd`, `LogicalOr` handling
  - OCaml `codegen.ml` chapter 4 relational/logical
  - Book: `docs/book/ch04-logical-and-relational-operators.md`, `docs/stages/ch04-logical-and-relational-operators.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { return 1 < 2; }` exits with 1.
  - [ ] `int main(void) { return 1 && 0; }` exits with 0.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 4 --latest-only --bitwise` reports every test passing.

  **QA Scenarios**:

  ```
  Scenario: Chapter 4 logical/relational + bitwise gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 4 --latest-only --bitwise (expect green)
    Failure Indicators: short-circuit incorrect -> fix `lower.rs::ast_to_tacky`; conditional setCC wrong -> fix codegen
    Evidence: .omo/evidence/task-17-ch4-gate.txt
  ```

- [x] 18. W6-T1: Chapter 5 - local variables, assignment, lvalues + `--compound` extra (`+= -= *= /= %= &= |= ^= <<= >>=`) + `--increment` extra (`++ --` prefix/postfix)

  **What to do**: Add variable declarations, assignment, compound assignment extras, and pre/post-increment/decrement.
  - Extend `src/ast/decl.rs`: `BlockItem = Decl(VarDecl) | Stmt(Stmt)`; `VarDecl { name: String, ty: Type, init: Option<Expr> }`. Note: ch.5 has no initializers (those land ch.11); use `None`.
  - Extend `src/ast/expr.rs`: `Var(String)`, `Assignment { lvalue: Box<Expr>, rvalue: Box<Expr> }`, compound `Assignment { lvalue, op: CompoundOp, rvalue }`, `Increment { op: IncrDecrOp, lvalue, prefix: bool }`.
  - Extend `src/parse/parser.rs::parse_block_item` to handle declarations; `parse_expression` to handle assignments and prefix/postfix `++`/`--`.
  - Add `src/semantics/resolve.rs::resolve_program` for chapter 5: tracks variable names in function scope. Stubs out for now; actually implement `fn resolve_block` in this task.
  - Extend `src/ir/lower.rs::ast_to_tacky`: declarations -> no TACKY instruction (variable is just a stack slot); assignments -> `Copy { src: rvalue_val, dst: Var(name) }`; compound assignments -> evaluate lvalue once into tmp, emit binary op, `Copy` back. Increment/decrement: prefix `++x` => `add $1, %tmp; copy to x`; postfix `x++` => `copy x to tmp; add $1, %tmp; copy tmp to x; tmp = old x`.
  - Extend `src/codegen/codegen.rs::generate`: lower `Copy(dst=Var(n))` to `mov src_var_or_imm, var_n_stack_slot`. Compound ops `+= -= *= /= %= &= |= ^= <<= >>=`: `load var to %eax`, `op %eax, src`, `store %eax to var`. Add assembly `AllocateStack(slot_size)`; introduce `FrameLayout` per function.
  - For chapter 5, gate is `--chapter 5 --latest-only --bitwise --compound --increment`.

  **Must NOT do**: Do not yet add `static`/`extern` - chapter 10. Do not add initializers - chapter 11. Do not add goto - chapter 6 extra.

  **Recommended Agent Profile**: Category `deep`. Lvalue semantics + compound + pre/post increment are subtle; `lvalue evaluated only once` requirement.

  **Parallelization**: Sequential after W5-T1.

  **References**:
  - OCaml `parse.ml` chapter 5 ~lines 280-450
  - OCaml `tacky_gen.ml` chapter 5 (declarations, assignments, compound ops, ++/--)
  - OCaml `codegen.ml` chapter 5 (`AllocateStack`, `mov src, [rbp-offset]`)
  - OCaml `resolve.ml` chapter 5 (initial scope-tracking pass)
  - Book: `docs/book/ch05-local-variables.md`, `docs/stages/ch05-local-variables.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int x = 5; return x; }` -> exit 5.
  - [ ] `int main(void) { int x = 5; x += 3; return x; }` -> exit 8.
  - [ ] `int main(void) { int x = 5; return ++x; }` -> exit 6.
  - [ ] `int main(void) { int x = 5; int y = x++; return y * 10 + x; }` -> exit `57`.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only --bitwise --compound --increment` green.

  **QA Scenarios**:

  ```
  Scenario: Chapter 5 local vars + compound + increment gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only --bitwise --compound --increment (expect green)
    Failure Indicators: lvalue evaluated twice -> fix `Compound`/`Increment` lowering
    Evidence: .omo/evidence/task-18-ch5-gate.txt
  ```

- [x] 19. W6-T2: Chapter 5 - scope shadowing + nested blocks (deferred to W8 with chapter 7). Update COACHING_LOG + commit.

  **Recommended Agent Profile**: `quick`.

  **References**: Same as W6-T1; `docs/COACHING_LOG.md`.

  **Acceptance Criteria**: gate green; log updated; commit made.

- [x] 20. W7-T1: Chapter 6 - `if`/`else`/ternary `?:` + `--goto` extra (labeled statements and `goto`)

  **What to do**: Add `if`, `if/else`, conditional expressions, and the goto extra.
  - Extend `src/ast/stmt.rs` with `If { condition, then_branch, else_branch: Option<Vec<Stmt>> }` and `Label { name: String, stmt: Box<Stmt> }` (extra) and `Goto(String)` (extra).
  - Extend `src/parse/parser.rs::parse_statement` for `if (...) stmt` and `if (...) stmt else stmt`. Extend `parse_expression` for `cond ? then_expr : else_expr` (right-associative).
  - Extend `src/ir/lower.rs::ast_to_tacky`: `if (c) then` => evaluate `c`, `JumpIfZero c, else_label`, emit then-body, `Jump end_label`, `Label else_label`, emit else-body, `Label end_label`. For goto: `Label(l) stmt` => emit `Label(l)` before stmt; `Goto(l)` => emit `Jump("l")`. For labeled loops (ch.8): use the labeled-loops infrastructure.
  - Implement `src/semantics/label_loops.rs::label_loops` to validate that every `goto` references an in-scope label. Reject duplicate labels.
  - Extend `src/codegen/codegen.rs::generate` if any new assembly instructions needed; chapter 6 mostly uses existing `Jmp`, `Label`, `JumpIfZero`, `JumpIfNotZero`.

  **Must NOT do**: Do not add loops - chapter 8. Do not add `switch` - chapter 8 extra.

  **Recommended Agent Profile**: Category `deep`. Conditional lowering + label validation have edge cases.

  **Parallelization**: Sequential after W6-T2.

  **References**:
  - OCaml `parse.ml` chapter 6 ~lines 450-580
  - OCaml `tacky_gen.ml` chapter 6 (`If`, `Goto`, ternary)
  - OCaml `label_loops.ml` (chapter 6 label validation)
  - Book: `docs/book/ch06-if-statements-and-conditional-expressions.md`, `docs/stages/ch06-if-and-conditional-expressions.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int x = 5; if (x > 0) return 10; else return 20; }` -> exit 10.
  - [ ] `int main(void) { int a = 1; a = a > 0 ? 7 : 8; return a; }` -> exit 7.
  - [ ] `int main(void) { int x = 0; goto end; x = 5; end: return x; }` -> exit 0.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 6 --latest-only --bitwise --compound --increment --goto` green.

  **QA Scenarios**:

  ```
  Scenario: Chapter 6 if/else/ternary/goto gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 6 --latest-only --bitwise --compound --increment --goto (expect green)
    Failure Indicators: cross-function goto accepted -> fix label_loops validation
    Evidence: .omo/evidence/task-20-ch6-gate.txt
  ```

- [x] 21. W7-T2: Chapter 6 - regression `chapter 5` core (no extras) still green. COACHING_LOG + commit.

  **What to do**: After W7-T1 lands, run `./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only` (no extras) to confirm chapter 5 has no regression; chapter 5's gate was verified in W6-T1 but `--latest-only` isolated it - this rerun verifies the chapter 6 parser + AST extensions do not break the chapter 5 subset.

  **Must NOT do**: Do not modify compiler code; this task is observation only.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W7-T1.

  **References**: `docs/book/test-map.md` row for chapter 5.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only` exits 0 with zero `fail` lines.

  **QA Scenarios**:

  ```
  Scenario: Chapter 5 regression after Chapter 6 lands
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only (expect green)
    Evidence: .omo/evidence/task-21-ch5-regression.txt
  ```


  **Recommended Agent Profile**: `quick`.

- [x] 22. W8-T1: Chapter 7 - compound statements, nested blocks, shadowing

  **What to do**: Add support for `{ }` blocks with their own scope; allow variable shadowing in inner scopes.
  - Extend `src/ast/stmt.rs`: `Block(Vec<BlockItem>)` and `BlockItem = Decl | Stmt`.
  - Extend `src/parse/parser.rs::parse_compound_statement` (ch.7 grammar).
  - Extend `src/semantics/resolve.rs::resolve_program` with a per-block scope stack: entering a block pushes a new scope, declarations add to current scope, name lookup walks the stack. Shadowing: inner-scope name shadows outer-scope.
  - Codegen: push a new `FrameLayout` for the block; allocations accumulate; on block exit, allocations are released (use the OCaml-equivalent frame-balance pattern).

  **Must NOT do**: Do not add loops - chapter 8.

  **Recommended Agent Profile**: Category `unspecified-high`. Scope management with shadowing.

  **Parallelization**: Sequential after W7-T2.

  **References**:
  - OCaml `parse.ml` chapter 7 ~lines 580-650
  - OCaml `resolve.ml` chapter 7 (per-block scope)
  - Book: `docs/book/ch07-compound-statements.md`, `docs/stages/ch07-compound-statements.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int x = 1; { int x = 5; } return x; }` -> exit 1 (inner shadow did not leak).
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only --compound --goto` green.

  **QA Scenarios**:

  ```
  Scenario: Chapter 7 compound statements gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only --compound --goto (expect green)
    Failure Indicators: shadowing not respected -> fix `resolve.rs` scope stack
    Evidence: .omo/evidence/task-22-ch7-gate.txt
  ```

- [x] 23. W8-T2: Chapter 7 gate verification + commit.

  **What to do**: Run the full chapter 7 gate with extras (`--compound --goto`), update `docs/COACHING_LOG.md`, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W8-T1.

  **References**: `docs/book/test-map.md` chapter 7 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only --compound --goto` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 8 (Chapter 7) entry written.
  - [ ] `git log --oneline -1` shows the chapter 7 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 7 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only --compound --goto (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 7: compound statements"
    Evidence: .omo/evidence/task-23-ch7-gate.txt
  ```


  **Recommended Agent Profile**: `quick`.

- [x] 24. W9-T1: Chapter 8 - loops (`while`, `do`, `for`, `break`, `continue`) + `--switch` extra (`switch`/`case`/`default`/`break`)

  **What to do**: Add loops and switch.
  - Extend `src/ast/stmt.rs`: `While { condition, body, label }`, `Do { body, condition, label }`, `For { init, condition, post, body, label }`, `Break(String)`, `Continue(String)`, `Switch { condition, body, label }`.
  - Extend `src/parse/parser.rs::parse_statement` for `while`, `do`, `for`.
  - Extend `src/semantics/label_loops.rs` to track label IDs for each loop (so `break L;` targets the right loop).
  - Extend `src/ir/lower.rs::ast_to_tacky`:
    - `while (c) body`: `start: JumpIfNotZero c, body_label; jump end; body_label: <body>; jump start; end:`
    - `do { body } while (c)`: body first, then `JumpIfNotZero c, body_label`; ... similar.
    - `for (init; cond; post) body`: init first, then `start: JumpIfZero cond, end; body; post; jump start; end:`.
    - `break L;` => `Jump L_break_label`; `continue L;` => `Jump L_continue_label`.
    - `switch (c) body`: collect case values + default; lower to a chain of `if (c == n1) goto case_n1_label; if (c == n2) ...` for jump-table style, or direct compare-jump chain. Book uses compare-jump chain (`nqcc2/lib/tacky_gen.ml` Switch).
  - Codegen: existing labels/jumps; no new assembly ops.

  **Must NOT do**: Do not add fall-through between cases without `break` semantics - that lands with `--switch`.

  **Recommended Agent Profile**: Category `deep`. Switch + loops' label tracking are tricky.

  **Parallelization**: Sequential after W8-T2.

  **References**:
  - OCaml `parse.ml` chapter 8 ~lines 650-820
  - OCaml `tacky_gen.ml` chapter 8 (`While`, `Do`, `For`, `Switch`)
  - OCaml `label_loops.ml` extension for loops
  - OCaml `codegen.ml` chapter 8 (no new ops)
  - Book: `docs/book/ch08-loops.md`, `docs/stages/ch08-loops.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int i = 0; int s = 0; while (i < 10) s += i++; return s; }` -> exit 45.
  - [ ] `int main(void) { int x = 1; switch (x) { case 0: return 0; case 1: return 1; case 2: return 2; default: return 99; } }` -> exit 1.
  - [ ] `int main(void) { int x = 0; for (int i = 0; i < 5; i++) { if (i == 3) break; x += i; } return x; }` -> exit `3`.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch` green.

  **QA Scenarios**:

  ```
  Scenario: Chapter 8 loops + switch gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch (expect green)
    Failure Indicators: switch fall-through wrong -> fix case dispatch
    Evidence: .omo/evidence/task-24-ch8-gate.txt
  ```

- [x] 25. W9-T2: Chapter 8 - `break`/`continue` correctness + Duff's device fallthrough with `--switch`. Update code accordingly.

  **What to do**: Verify `break` exits the right loop, `continue` jumps to the loop's continue label, and switch-case fallthrough without break is honored.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Parallel with W9-T1.

  **Acceptance Criteria**: Switch/loops test gates are clean (already covered by W9-T1).

- [x] 26. W9-T3: Chapter 8 gate rerun + COACHING_LOG + commit.

  **What to do**: Run the full chapter 8 gate with all relevant extras (`--compound --increment --goto --switch`), update `docs/COACHING_LOG.md`, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W9-T2.

  **References**: `docs/book/test-map.md` chapter 8 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 9 (Chapter 8) entry written.
  - [ ] `git log --oneline -1` shows the chapter 8 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 8 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 8: loops + switch"
    Evidence: .omo/evidence/task-26-ch8-gate.txt
  ```


  **Recommended Agent Profile**: `quick`.

- [x] 27. W10-T1: Chapter 9 - reshape AST from single-function `Program` to multi-function `Program` (the BIG pivot)

  **What to do**: Reshape `Program` to hold multiple top-level functions. Updates ripple through parser, sema, TACKY, codegen.
  - `src/ast/item.rs`: change `pub struct Program` from `{ function }` to `{ pub top_level_items: Vec<TopLevelItem> }`. `TopLevelItem = Function(Function) | StaticVar(...)` (the latter for ch.10, but stub now).
  - `src/parse/parser.rs::parse_program`: instead of parsing exactly one function, loop `parse_top_level_item` until EOF. Each item is either a function definition or a declaration with no body.
  - Update `src/parse/parser.rs::parse_function` to parse parameter list `(int x, int y, ...)` returning `Function { name, params: Vec<VarDecl>, body }`. The `void` empty-parameter form is preserved.
  - Update `src/semantics/resolve.rs` (real implementation, not stub): per `nqcc2/lib/semantic_analysis/resolve.ml`. Add a global function table. Track parameter names in function scope. Reject conflicting declarations; reject duplicate definitions.
  - `src/ir/lower.rs::ast_to_tacky`: emit one `TackyFunction` per AST `Function`. Multiple-function `TackyProgram`.
  - `src/codegen/codegen.rs::generate`: emit one `TopLevel::Fn` per TACKY function. `main` is still a function; other functions are emitted too with appropriate `.globl`/no-`.globl`.

  **Must NOT do**: Do not add parameter passing yet (that's W10-T2). Do not add extern/global declarations (that's ch.10).

  **Recommended Agent Profile**: Category `deep`. The AST pivot is non-trivial; every module that touches `Program` must change.

  **Parallelization**: Sequential gate for the chapter 9 pivot. Blocked By: W9-T3.

  **References**:
  - OCaml `parse.ml` `parse_translation_unit` ~line 1-50 (ch.9)
  - OCaml `ast.ml` `Program` change (multi-function)
  - OCaml `resolve.ml` (chapter 9) - the global function table
  - OCaml `tacky_gen.ml` (chapter 9) - one TackyFunction per AST function
  - OCaml `codegen.ml` (chapter 9) - emit multiple `Fn { ... }` top-levels
  - Book: `docs/book/ch09-functions.md`, `docs/stages/ch09-functions.md`
  - Current (single-func) AST: `src/ast/item.rs:1-13`

  **Acceptance Criteria**:
  - [ ] `src/ast/item.rs::Program` is `{ top_level_items: Vec<TopLevelItem> }`.
  - [ ] A two-function program (`int f(void) { return 1; } int main(void) { return f(); }`) parses without errors.
  - [ ] `cargo check --release` zero errors.

  **QA Scenarios**:

  ```
  Scenario: Multi-function AST pivots cleanly
    Tool: Bash (cargo)
    Preconditions: All Wave 0-9 tasks complete
    Steps:
      1. Write /tmp/multi.c containing `int f(void) { return 1; } int main(void) { return f(); }`
      2. cargo build --release
      3. target/release/rustcc /tmp/multi.c --stage parse (expect multi-function AST dump)
      4. cargo check --release (expect exit 0)
    Failure Indicators: missing top-level item -> extend parser; visitor hits old field -> check ast/item.rs
    Evidence: .omo/evidence/task-27-ch9-ast-pivot.txt
  ```

- [x] 28. W10-T2: Chapter 9 - parameter passing via ABI + function calls

  **What to do**: Implement ABI for argument passing and call/return.
  - Add `src/codegen/abi.rs::classify_params(params: &[VarDecl]) -> AbiPlan`: assign each parameter to integer (`%rdi, %rsi, %rdx, %rcx, %r8, %r9`) or SSE register (chapter 13) or stack-passed (chapter 9+).
  - In `src/codegen/codegen.rs::generate_function` (chapter 9): prologue moves incoming args from registers into the function's local stack slots; for `>=7` args, copy from stack.
  - Add TACKY `Instruction::Call { name: String, args: Vec<Val>, dst: Option<Var> }`.
  - In `src/codegen/codegen.rs`: lower `Call` to `push` args in reverse order (if stack-passed), `mov` integer args to registers (`%rdi, %rsi, ...`), `call <name>`, `mov %rax, dst` (if dst is set).
  - Stack alignment must be maintained (16-byte alignment before `call` per System V). Add comment explaining.

  **Must NOT do**: Do not pass struct args yet (chapter 18). Do not pass float args via XMM yet (chapter 13).

  **Recommended Agent Profile**: Category `deep`. ABI is hard to get right and subtle.

  **Parallelization**: Parallel with W10-T3. Blocked By: W10-T1.

  **References**:
  - OCaml `codegen.ml` chapter 9 (parameter passing, call emission, stack alignment)
  - OCaml `abi.rs` to mirror (`classify_params`)
  - System V AMD64 ABI doc: https://refspecs.linuxfoundation.org/elf/x86_64-abi-0.99.pdf
  - Book: `docs/book/ch09-functions.md`

  **Acceptance Criteria**:
  - [ ] `int add(int a, int b) { return a + b; } int main(void) { return add(2, 3); }` -> exit 5.
  - [ ] `int main(void) { return foo(7); } int foo(int x) { return x + 3; }` -> exit 10.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 9 --latest-only --bitwise --compound --increment --goto --switch` green.

  **QA Scenarios**:

  ```
  Scenario: Chapter 9 multi-function + ABI gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 9 --latest-only --bitwise --compound --increment --goto --switch (expect green)
    Failure Indicators: stack misalignment (test crashes) -> fix 16-byte alignment; arg swapped -> fix register order
    Evidence: .omo/evidence/task-28-ch9-abi-gate.txt
  ```

- [x] 29. W10-T3: Chapter 9 - `src/codegen/fixup.rs` real implementation (callee-saved save/restore, push/pop alignment)

  **What to do**: Replace the no-op fixup with real instruction fixup for chapter 9+.
  - For each function, determine which callee-saved registers (`%rbx, %r12-%r15`) are used.
  - Insert prologue: `push %rbx; push %r12; ...` for used callee-saved; insert epilogue: `pop` inverse order.
  - Stack alignment: ensure `rsp` is 16-byte aligned before any `call`. This may require an `AllocateStack` adjustment.
  - Convert illegal memory-to-memory instructions: `mov [mem], [mem]` -> `mov [mem], %rax; mov %rax, [mem]`.
  - Mirror the OCaml `nqcc2/lib/backend/instruction_fixup.ml` exactly.

  **Must NOT do**: Do not implement register allocation - chapter 20. Do not handle double register class - chapter 13.

  **Recommended Agent Profile**: Category `deep`. Callee-saved tracking and instruction legalisation are subtle.

  **Parallelization**: Parallel with W10-T2. Blocked By: W10-T1.

  **References**:
  - OCaml `instruction_fixup.ml:1-251`
  - System V AMD64 ABI
  - Book: `docs/book/ch09-functions.md`

  **Acceptance Criteria**:
  - [ ] Functions using callee-saved registers still produce correct results after fixup prologue/epilogue.
  - [ ] All chapter-9 tests still pass after fixup is real.

- [x] 30. W10-T4: Chapter 9 - gate verification + cross-check multi-function behavior

  **What to do**: Run full chapter 9 gate; manually verify with a 3-function program; commit.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Acceptance Criteria**: Gate green; multi-function program runs correctly; commit made.

- [x] 31. W11-T1: Chapter 10 - global/static variables, `extern`, linkage, `.data` and `.bss` emission

  **What to do**: Add file-scope variables (global, static, extern).
  - Extend `src/ast/item.rs::TopLevelItem` with `Var(GlobalDecl { name, ty, init, storage: StorageClass })`.
  - Extend `src/semantics/resolve.rs` to track global symbol table (separate from function scope).
  - Extend `src/ir/tacky.rs` with top-level `StaticVariable { name, init }` and `StaticConstant`.
  - Extend `src/codegen/codegen.rs::generate_program` to emit `.globl` (or not, if `static`), `.data` (initialized) or `.bss` (zero-init), with appropriate alignment.
  - Extend the ABI/calling convention: cross-file function calls must work (since `extern` declares a function defined elsewhere).

  **Must NOT do**: Do not yet add initializers for globals (chapter 11). Do not yet track alignment beyond default.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Sequential after W10-T4.

  **References**:
  - OCaml `parse.ml` chapter 10 ~lines 1-100 (top-level declarations)
  - OCaml `codegen.ml` chapter 10 (`.data`/`.bss` emission)
  - OCaml `resolve.ml` chapter 10
  - Book: `docs/book/ch10-file-scope-variables-and-storage-class-specifiers.md`, `docs/stages/ch10-globals-and-storage-classes.md`

  **Acceptance Criteria**:
  - [ ] `static int g = 5; int main(void) { return g; }` -> exit 5.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only` green.

  **QA Scenarios**:

  ```
  Scenario: Chapter 10 globals + linkage gate
    Tool: Bash (Python harness)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only (expect green)
    Failure Indicators: extern vs static linkage mixup -> fix storage-class classification
    Evidence: .omo/evidence/task-31-ch10-gate.txt
  ```

- [x] 32. W11-T2: Chapter 10 - gate verification + commit.

  **What to do**: Run the full chapter 10 gate (no extras in this chapter), update `docs/COACHING_LOG.md`, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W11-T1.

  **References**: `docs/book/test-map.md` chapter 10 row; `docs/COACHING_LOG.md`.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 11 (Chapter 10) entry written.
  - [ ] `git log --oneline -1` shows the chapter 10 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 10 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 10: file-scope variables + linkage"
    Evidence: .omo/evidence/task-32-ch10-gate.txt
  ```


- [x] 33. W12-T1: Chapter 11 - `long` 64-bit integers (foundation infra: `Const`, `type_utils`, `assembly_symbols`)

  **What to do**: Add `long` (64-bit signed).
  - Add `src/codegen/const.rs` (`pub enum ConstValue { Int(i64), Long(i64), UInt(u64), ULong(u64), Double(f64) }`) mirror `nqcc2/lib/const.ml` (chapter 11 introduces this).
  - Extend AST `Type` with `Long` variant.
  - Extend `src/semantics/typecheck.rs::typecheck` (chapter 11+): unify int with long via `usual arithmetic conversions`.
  - Extend TACKY `Instruction::SignExtend`, `Truncate`.
  - Extend codegen: `movq`, `cqo`, `idivq` for `long` operations. Stack slot size doubles (`AllocateStack(8)` per local).

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Sequential after W11-T2.

  **References**:
  - OCaml `const.ml` (chapter 11)
  - OCaml `typecheck.ml` chapter 11 (`usual_arithmetic_conversions`)
  - OCaml `codegen.ml` chapter 11 (long integer ops)
  - Book: `docs/book/ch11-long-integers.md`, `docs/stages/ch11-long-integers.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { long x = 10000000000L; return x > 0; }` -> exit 1.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 11 --latest-only` green.

- [x] 34. W12-T2: Chapter 11 - width-aware codegen, `SignExtend` for int<->long. Gate verification + commit.

  **What to do**: Confirm all chapter-11-conditional SignExtend lowering produces correct results; run `--chapter 11 --latest-only`; update COACHING_LOG; commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W12-T1.

  **References**: `docs/book/test-map.md` chapter 11 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 11 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 12 (Chapter 11) entry written.
  - [ ] `git log --oneline -1` shows the chapter 11 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 11 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 11 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 11: long integers"
    Evidence: .omo/evidence/task-34-ch11-gate.txt
  ```


- [x] 35. W13-T1: Chapter 12 - `unsigned int` and `unsigned long` (zero-extension, unsigned comparisons)

  **What to do**: Add unsigned integers.
  - Extend AST `Type` with `UInt`, `ULong`.
  - Extend `src/codegen/const.rs` with `UInt`, `ULong`.
  - Extend TACKY with `ZeroExtend`.
  - Extend codegen: `movl src, dst` for unsigned; use unsigned-conditional-set (`seta`, `setae`, `setb`, `setbe` for above-vs-below).
  - Implement signed-vs-unsigned promotion rules.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Sequential after W12-T2.

  **References**:
  - OCaml `typecheck.ml` chapter 12 (unsigned conversions)
  - OCaml `codegen.ml` chapter 12
  - Book: `docs/book/ch12-unsigned-integers.md`, `docs/stages/ch12-unsigned-integers.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { unsigned int x = 5u; return x > 0u; }` -> exit 1.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 12 --latest-only` green.

- [x] 36. W13-T2: Chapter 12 - gate verification + commit.

  **What to do**: Run chapter 12 gate (unsigned int + unsigned long), update COACHING_LOG, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W13-T1.

  **References**: `docs/book/test-map.md` chapter 12 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 12 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 13 (Chapter 12) entry written.
  - [ ] `git log --oneline -1` shows the chapter 12 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 12 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 12 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 12: unsigned integers"
    Evidence: .omo/evidence/task-36-ch12-gate.txt
  ```


- [x] 37. W14-T1: Chapter 13 - `double` floats, XMM register file (foundation for SSE), `DoubleToInt`, `IntToDouble`

  **What to do**: Add `double` floats. This chapter introduces the second register class.
  - Extend AST `Type` with `Double`.
  - Extend `src/codegen/const.rs` with `Double(f64)`. Use `HashMap` of doubles -> their `.rodata` labels (per OCaml pattern).
  - Extend TACKY with `DoubleConstant(f64)`, `IntToDouble`, `DoubleToInt`, `UIntToDouble`, `DoubleToUInt`. Add `Movsd` (scalar double) and `MovDouble` variants for TACKY.
  - Extend codegen: SSE instructions (`movsd src, dst`, `addsd`, `subsd`, `mulsd`, `divsd`, `ucomisd`, `cvttsd2si`).
  - Add double parameter-passing registers (`%xmm0-%xmm7`).

  **Must NOT do**: Do not pass structs containing doubles yet (chapter 18). Do not yet handle NaN (chapter 13 extra).

  **Recommended Agent Profile**: Category `deep`.

  **Parallelization**: Sequential after W13-T2. Major pivot.

  **References**:
  - OCaml `const.ml` chapter 13 (DoubleConstant + .rodata label table)
  - OCaml `codegen.ml` chapter 13 (SSE register file + double ops)
  - OCaml `assembly.ml` extension for XMM registers
  - Book: `docs/book/ch13-floating-point-numbers.md`, `docs/stages/ch13-floating-point.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { double x = 3.5; return x > 1.0; }` -> exit 1.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only` (core, no NaN) green.

- [x] 38. W14-T2: Chapter 13 - NaN-aware comparisons (`--nan` extra)

  **What to do**: Implement NaN handling in float comparisons.
  - Per book, NaN comparisons always produce `false` for `==` and `true` for `!=`. `<`, `<=`, `>`, `>=` all return `false` for any NaN operand.
  - Codegen for `ucomisd`: set ZF=1 for unordered. Combine with other flags to produce `setCC` masks that handle both ordered and unordered cases.

  **Recommended Agent Profile**: Category `deep`. NaN semantics are subtle.

  **Parallelization**: Parallel with W14-T3.

  **References**: OCaml `codegen.ml` chapter 13 NaN handling.

  **Acceptance Criteria**:
  - [ ] `int main(void) { double x = 0.0/0.0; return x != x; }` -> exit 1.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan` green.

- [x] 39. W14-T3: Chapter 13 - gate verification + commit.

  **What to do**: Run all chapter 13 gates: core (`--chapter 13 --latest-only`), `--nan` extra (`--chapter 13 --latest-only --nan`); update COACHING_LOG; commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W14-T2.

  **References**: `docs/book/test-map.md` chapter 13 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only` exits 0.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 14 (Chapter 13) entry written.
  - [ ] `git log --oneline -1` shows the chapter 13 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 13 gate green (core + NaN) + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only (expect green)
      3. ./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan (expect green)
      4. git add -A; git commit -m "feat(compiler): chapter 13: doubles + NaN"
    Evidence: .omo/evidence/task-39-ch13-gate.txt
  ```


- [x] 40. W15-T1: Chapter 14 - pointers (`*` deref, `&` address-of, pointer comparison)

  **What to do**: Add pointers.
  - Extend AST `Type` with `Pointer(Box<Type>)`.
  - Extend `src/ast/expr.rs` with `AddressOf(Box<Expr>)` and `Dereference(Box<Expr>)`. Add `Assignment` where lvalue can be `*p = ...`.
  - Extend `src/semantics/typecheck.rs::typecheck` (chapter 14): `&x` requires x to be an lvalue; `*p` requires p to be a pointer.
  - Extend TACKY with `GetAddress { src: Var, dst: Var }`, `Load { src_pointer: Val, dst: Var }`, `Store { src: Val, dst_pointer: Val }`.
  - Extend codegen: `lea` for `&`, `mov` indirection for `*`.

  **Recommended Agent Profile**: Category `deep`. Pointer semantics and lvalue tracking.

  **Parallelization**: Sequential after W14-T3.

  **References**:
  - OCaml `ast.ml` chapter 14
  - OCaml `parse.ml` chapter 14 (declarators with `*`)
  - OCaml `typecheck.ml` chapter 14
  - OCaml `tacky_gen.ml` chapter 14 (`GetAddress`, `Load`, `Store`)
  - OCaml `codegen.ml` chapter 14 (`lea`)
  - Book: `docs/book/ch14-pointers.md`, `docs/stages/ch14-pointers.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int x = 5; int *p = &x; return *p; }` -> exit 5.
  - [ ] `int main(void) { int x = 5; int *p = &x; *p = 10; return x; }` -> exit 10.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only` green.

- [x] 41. W15-T2: Chapter 14 - gate verification + commit.

  **What to do**: Run chapter 14 gate, update COACHING_LOG, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W15-T1.

  **References**: `docs/book/test-map.md` chapter 14 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 15 (Chapter 14) entry written.
  - [ ] `git log --oneline -1` shows the chapter 14 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 14 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 14: pointers"
    Evidence: .omo/evidence/task-41-ch14-gate.txt
  ```


- [x] 42. W16-T1: Chapter 15 - arrays and pointer arithmetic (`[]`, decay)

  **What to do**: Add arrays.
  - Extend AST `Type` with `Array { element: Box<Type>, size: Option<usize> }` (size optional for parameters).
  - Add `Subscript { base, index }` to expr.
  - Add `AddPtr` TACKY (pointer + index*size).
  - Array decay: when an array is passed to a function, it decays to a `T*` pointer.
  - Codegen: `[]` lowers to `*(arr + i*size)`; `mov` with effective-address arithmetic.

  **Recommended Agent Profile**: Category `deep`.

  **Parallelization**: Sequential after W15-T2.

  **References**:
  - OCaml `ast.ml` chapter 15 (Array type)
  - OCaml `parse.ml` chapter 15 (declarator processing for `int a[10]`)
  - OCaml `tacky_gen.ml` chapter 15 (`AddPtr`, `Subscript`)
  - OCaml `codegen.ml` chapter 15
  - Book: `docs/book/ch15-arrays-and-pointer-arithmetic.md`, `docs/stages/ch15-arrays-and-pointer-arithmetic.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int a[3]; a[0] = 10; a[1] = 20; a[2] = 30; return a[0] + a[1] + a[2]; }` -> exit 60.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only` green.

- [x] 43. W16-T2: Chapter 15 - gate verification + commit.

  **What to do**: Run chapter 15 gate, update COACHING_LOG, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W16-T1.

  **References**: `docs/book/test-map.md` chapter 15 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 16 (Chapter 15) entry written.
  - [ ] `git log --oneline -1` shows the chapter 15 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 15 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 15: arrays and pointer arithmetic"
    Evidence: .omo/evidence/task-43-ch15-gate.txt
  ```


- [x] 44. W17-T1: Chapter 16 - characters and string literals

  **What to do**: Add `char` and string/char literals.
  - Extend AST `Type` with `Char`.
  - Add Token kinds for char literals (e.g. `'a'`), string literals (e.g. `"hello"`).
  - Add char constants to AST and TACKY `Val` enum.
  - String literals: stored in `.rodata` as `.ascii` or null-terminated. The string identifier is `TackyProgram::StaticConstant { label, init: bytes }`.
  - Codegen for string literal: emit `.section .rodata; .byte 0x..., 0;` per the book.
  - Add `puts` library call (assumed linked via gcc).

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Sequential after W16-T2.

  **References**:
  - OCaml `lex.ml` chapter 16 (char/string lexing)
  - OCaml `tacky_gen.ml` chapter 16
  - OCaml `codegen.ml` chapter 16 (`.rodata` section)
  - Book: `docs/book/ch16-characters-and-strings.md`, `docs/stages/ch16-characters-and-strings.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { char c = 'A'; return c; }` -> exit 65.
  - [ ] `int main(void) { char *s = "hello"; return s[0]; }` -> exit 104.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` green.

- [x] 45. W17-T2: Chapter 16 - gate verification + commit.

  **What to do**: Run chapter 16 gate (chars and string literals), update COACHING_LOG, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W17-T1.

  **References**: `docs/book/test-map.md` chapter 16 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 17 (Chapter 16) entry written.
  - [ ] `git log --oneline -1` shows the chapter 16 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 16 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 16: chars and strings"
    Evidence: .omo/evidence/task-45-ch16-gate.txt
  ```


- [x] 46. W18-T1: Chapter 17 - `void`, `void*`, `sizeof`, dynamic memory (`malloc`/`free` declarations)

  **What to do**: Add `void` and `sizeof` operator.
  - Extend AST `Type` with `Void`.
  - Add `SizeOfExpr(Box<Expr>)` and `SizeOfType(Type)` to expr.
  - Update `src/semantics/typecheck.rs`: `sizeof` is `int` constant; `void` only as function return type or as part of `void*`; can declare functions returning `void`.
  - TACKY: `sizeof` is constant-folded; emit `movl $size, dst`.
  - `malloc`/`free` are not implemented; assume linked from libc.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Sequential after W17-T2.

  **References**:
  - OCaml `ast.ml` chapter 17 (Void, SizeOf)
  - OCaml `typecheck.ml` chapter 17 (sizeof handling)
  - OCaml `tacky_gen.ml` chapter 17
  - Book: `docs/book/ch17-supporting-dynamic-memory-allocation.md`, `docs/stages/ch17-dynamic-memory-support.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { return sizeof(int); }` -> exit 4.
  - [ ] `int main(void) { void *p = (void *)0; return p == 0; }` -> exit 1.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only` green.

- [x] 47. W18-T2: Chapter 17 - gate verification + commit.

  **What to do**: Run chapter 17 gate (sizeof, void, void*), update COACHING_LOG, commit.

  **Must NOT do**: Do not modify compiler code.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W18-T1.

  **References**: `docs/book/test-map.md` chapter 17 row.

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only` exits 0 with zero `fail` lines.
  - [ ] `docs/COACHING_LOG.md` Wave 18 (Chapter 17) entry written.
  - [ ] `git log --oneline -1` shows the chapter 17 commit.

  **QA Scenarios**:

  ```
  Scenario: Chapter 17 gate green + commit
    Tool: Bash (Python harness + git)
    Steps:
      1. cargo build --release
      2. ./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only (expect green)
      3. git add -A; git commit -m "feat(compiler): chapter 17: dynamic memory support"
    Evidence: .omo/evidence/task-47-ch17-gate.txt
  ```


- [x] 48. W19-T1: Chapter 18 - `struct` declarations, member access `.` and `->`, struct literals

  **What to do**: Add structs.
  - Extend AST `Type` with `Struct { name: String, fields: Vec<(String, Type)> }` (with empty name for forward decl).
  - Add `src/codegen/type_table.rs`: tracks struct definitions with size, alignment, member map. Mirror `nqcc2/lib/type_table.ml`.
  - Extend `src/parse/parser.rs`: parse `struct { ... }` and `struct_name.member`.
  - Extend `src/semantics/typecheck.rs`: validate field access on struct types; reject access on non-existent fields.
  - Extend TACKY with `GetElementPtr { src: StructVal, dst: Var, offset: usize }` (or equivalent).
  - Codegen: `struct` values in stack slots; member access => `[struct_addr + field_offset]`.

  **Must NOT do**: Do not add struct bitfields (not in book). Do not add unions yet (extra).

  **Recommended Agent Profile**: Category `deep`. Struct member layout is non-trivial.

  **Parallelization**: Sequential after W18-T2.

  **References**:
  - OCaml `type_table.ml:1-28`
  - OCaml `typecheck.ml` chapter 18
  - OCaml `tacky_gen.ml` chapter 18 (`GetElementPtr` or equivalent)
  - OCaml `codegen.ml` chapter 18
  - Book: `docs/book/ch18-structures.md`, `docs/stages/ch18-structures.md`

  **Acceptance Criteria**:
  - [ ] `struct s { int a; int b; }; int main(void) { struct s x; x.a = 5; x.b = 10; return x.a + x.b; }` -> exit 15.
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only` (core) green.

- [x] 49. W19-T2: Chapter 18 - `--union` extra (`union` types sharing storage)

  **What to do**: Add union extra.
  - Extend AST `Type` with `Union { name, fields }`.
  - Extend `src/codegen/type_table.rs` with union definitions; size = max field size, alignment = max field alignment.
  - Member access: same as struct, but all fields start at offset 0.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Parallel with W19-T3. Blocked By: W19-T1.

  **References**:
  - OCaml `type_table.ml` chapter 18 (Union handling)
  - OCaml `typecheck.ml` chapter 18 (Union validation)

  **Acceptance Criteria**:
  - [ ] `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` green.

- [x] 50. W19-T3: Chapter 18 - System V ABI struct classification (Integer/SSE/Memory eightbytes, up to two eightbytes in registers)

  **What to do**: Implement struct-passing ABI.
  - `src/codegen/abi.rs::classify_struct_arg`: determine if struct fits in 2 eightbytes; if yes, assign register classes per eightbyte.
  - Codegen for struct return: by register (if small enough) or hidden first parameter (pointer to caller-allocated space).

  **Must NOT do**: Do not add bitfields. Do not add C99 flexible array members.

  **Recommended Agent Profile**: Category `deep`. ABI classification is hard.

  **Parallelization**: Sequential; depends on W19-T1.

  **References**:
  - OCaml `codegen.ml` chapter 18 (struct argument classification - most complex codegen logic)
  - System V AMD64 ABI documents

  **Acceptance Criteria**: Full chapter 18 gate (`--union` extra) green; large structs (>16 bytes) returned via hidden arg.

- [x] 51. W20-T1: Chapter 19 - CFG construction (foundational for all 4 optimization passes)

  **What to do**: Build the CFG abstraction used by all 4 opt passes.
  - In `src/ir/cfg.rs::build<N>`: implement the generic CFG (functor in OCaml; generics in Rust).
  - CFG has nodes (basic blocks), edges (preds/successors), entry/exit.
  - Use a trait bound `N: Copy + AsRef<[Block]>` or similar minimal interface.
  - Build two CFG flavors: TACKY-CFG (for constant folding, copy prop, dead store elim, unreachable code elim) and assembly-CFG (used later for ch.20 liveness).

  **Recommended Agent Profile**: Category `deep`. Generic CFG with trait bounds.

  **Parallelization**: Sequential gate for W20 progress. Blocked By: W19-T3.

  **References**:
  - OCaml `cfg.ml:1-341` (functor over instruction type)
  - Book: `docs/book/ch19-optimizing-tacky-programs.md`, `docs/stages/ch19-optimizations.md`

  **Acceptance Criteria**:
  - [ ] `cargo check --release` zero errors.
  - [ ] W20-T2 through W20-T5 build atop this CFG.

- [x] 52. W20-T2: Chapter 19 - constant folding pass

  **What to do**: Implement constant folding.
  - In `src/ir/opt.rs::run_opt`, add `OptPass::ConstantFolding`.
  - For each TACKY instruction: if operands are both constants and the operation is a binary op, replace with the computed constant.
  - Constant evaluation via `src/ir/const_eval.rs` (new): mirrors OCaml `nqcc2/lib/optimizations/constant_folding.ml` (175 LOC) with its `ConstEvaluator` functor supporting `int/long/uint/ulong/double`.

  **Recommended Agent Profile**: Category `deep`.

  **Parallelization**: Parallel with W20-T3..T5 after W20-T1 is done.

  **References**:
  - OCaml `constant_folding.ml:1-175`
  - OCaml `optimize.ml` orchestration
  - Book: `docs/stages/ch19-optimizations.md`

  **Acceptance Criteria**:
  - [ ] `int main(void) { int x = 2; x = x + 3; return x; }` produces assembly where the load+add is replaced by `movl $5, x_slot`.
  - [ ] `--chapter 19 --latest-only --fold-constants` passes.

- [x] 53. W20-T3: Chapter 19 - unreachable code elimination

  **What to do**: Implement UCE.
  - In `src/ir/opt.rs::run_opt`, add `OptPass::UnreachableCodeElim`.
  - Remove blocks unreachable from the entry block.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Parallel with W20-T2, W20-T4, W20-T5.

  **References**: OCaml `unreachable_code_elim.ml`, OCaml `optimize.ml`.

  **Acceptance Criteria**:
  - [ ] `--chapter 19 --latest-only --eliminate-unreachable-code` passes.

- [x] 54. W20-T4: Chapter 19 - copy propagation

  **What to do**: Implement copy prop.
  - For each `Copy { src, dst }` where `src` is a constant, propagate the constant to uses of `dst`. Uses dataflow on the CFG.

  **Recommended Agent Profile**: Category `deep`.

  **Parallelization**: Parallel with W20-T2, W20-T3, W20-T5.

  **References**: OCaml `copy_prop.ml`, OCaml `backward_dataflow.ml` (47 LOC, functor).

  **Acceptance Criteria**:
  - [ ] `--chapter 19 --latest-only --propagate-copies` passes.

- [ ] 55. W20-T5: Chapter 19 - dead store elimination + default all-optimizations gate

  **What to do**: Implement DSE; verify all 4 passes work in combination.
  - For each store to a variable that is overwritten before being read, remove the store.

  **Recommended Agent Profile**: Category `deep`.

  **Parallelization**: Parallel with W20-T2..T4.

  **References**: OCaml `dead_store_elim.ml`.

  **Acceptance Criteria**:
  - [ ] `--chapter 19 --latest-only --eliminate-dead-stores` passes.
  - [ ] `--chapter 19 --latest-only` (default all 4 passes) passes.

- [ ] 56. W21-T1: Chapter 20 - liveness analysis via backward dataflow

  **What to do**: Implement liveness on the assembly CFG.
  - In `src/codegen/regalloc/mod.rs` (new module), write `pub fn liveness<N: ...>(cfg: &Cfg<N>) -> HashMap<BlockId, (LiveSet, LiveSet)>`. Mirror OCaml `liveness` from `regalloc.ml`.
  - Each pseudo-register in the assembly has its interference computed.

  **Recommended Agent Profile**: Category `deep`. Iterative dataflow with fixed-point.

  **Parallelization**: Sequential gate. Blocked By: W20-T5.

  **References**:
  - OCaml `regalloc.ml:1-651` (`liveness` module)
  - OCaml `address_taken.ml` (17 LOC, used by regalloc; computes which vars have `&` taken so they need to be spilled to memory)

  **Acceptance Criteria**:
  - [ ] For a small program, liveness output matches the OCaml reference.

- [ ] 57. W21-T2: Chapter 20 - interference graph + simplification

  **What to do**: Build interference graph.
  - `pub fn build_interference<N: ...>(cfg, liveness) -> InterferenceGraph`. Each node represents a pseudo-register; edges represent simultaneous liveness.
  - Simplification: `pub fn simplify<N>(graph: &mut InterferenceGraph) -> Vec<PseudoReg>`: repeatedly remove low-degree (<`K`) nodes. If no such node, pick a spill candidate.

  **Recommended Agent Profile**: Category `deep`. Graph algorithms.

  **Parallelization**: Sequential after W21-T1.

  **References**: OCaml `regalloc.ml`.

  **Acceptance Criteria**:
  - [ ] For a program with 5 variables used in different scopes, interference graph has at most the expected edges.

- [ ] 58. W21-T3: Chapter 20 - coloring + select phase (graph-coloring register allocation)

  **What to do**: Color the graph.
  - `pub fn select(graph, k) -> HashMap<PseudoReg, Option<Reg>>`. Iterate simplification stack in reverse; assign registers to nodes that were simplified without conflict. If a node's degree > k at selection time, mark for spilling.

  **Recommended Agent Profile**: Category `deep`.

  **Parallelization**: Sequential after W21-T2.

  **References**: OCaml `regalloc.ml` (`select`).

  **Acceptance Criteria**:
  - [ ] `--chapter 20 --latest-only --no-coalescing` passes.

- [ ] 59. W21-T4: Chapter 20 - spilling + re-allocation loop

  **What to do**: Handle spills.
  - For each spilled pseudo, assign a stack slot. Re-run allocation until no spills remain (or limit iterations).
  - Replace pseudoregisters with stack slots in the assembly via `src/codegen/replace_pseudos.rs` (real impl now).

  **Recommended Agent Profile**: Category `deep`. Spill+rewrite is iterative.

  **Parallelization**: Sequential after W21-T3.

  **References**: OCaml `replace_pseudos.ml:1-137`, OCaml `regalloc.ml` spill logic.

  **Acceptance Criteria**:
  - [ ] Program with many temporaries compiles without infinite loop. Spilled slots produce correct results.

- [ ] 60. W21-T5: Chapter 20 - conservative coalescing (Briggs/George) + both-modes gate

  **What to do**: Implement coalescing. When pseudo `a` and `b` are connected by a `Copy` and don't interfere, merge them.
  - Update `src/codegen/regalloc/mod.rs::allocate` to call into the coalescing pipeline when `--no-coalescing` is NOT passed.
  - `pub fn coalesce(graph, moves)`: Briggs approach (conservative; merge only if the resulting degree is < k after neighbors are pruned).
  - Run `--chapter 20 --latest-only` (with coalescing) and `--chapter 20 --latest-only --no-coalescing`.

  **Recommended Agent Profile**: Category `deep`. Briggs algorithm is non-trivial.

  **Parallelization**: Sequential after W21-T4.

  **References**: OCaml `regalloc.ml` `coalesce`.

  **Acceptance Criteria**:
  - [ ] `--chapter 20 --latest-only --no-coalescing` passes.
  - [ ] `--chapter 20 --latest-only` (with coalescing) passes.
  - [ ] For program with many `Copy` instructions between unrelated variables, coalesced output uses fewer registers than non-coalesced.

- [ ] 61. W22-T1: Wave 22 - README + invocation update + tooling polish

  **What to do**: Update documentation to reflect completed status.
  - Update `README.md` to remove stale `chapter N` claim on the binary (it doesn't take `--chapter`; only `test_compiler` does); show real usage.
  - Update `docs/COACHING_LOG.md` final entry: all 20 chapters' gates recorded.
  - Confirm no `cargo build --release` warnings (post-cleanup).
  - Confirm `cargo test --release` 9 unit tests pass.
  - Lint with `cargo clippy --release` (no warnings).

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential after W21-T5.

  **Acceptance Criteria**:
  - [ ] `README.md` accurately documents `target/release/rustcc <input.c>` invocation.
  - [ ] `cargo build --release` zero warnings.
  - [ ] `cargo clippy --release -- -W clippy::all` zero clippy warnings.
  - [ ] `docs/COACHING_LOG.md` final entry lists all 20 chapter gate commands run.

- [ ] 62. W22-T2: Wave 22 - full regression + integration check

  **What to do**: Run all 20 chapter gates cumulatively (with extras), then a `--chapter N` (no `--latest-only`) regression on `N ∈ {5, 10, 15, 18, 20}`.
  - For each `N` in 1..=18: `./tests/test_compiler ./target/release/rustcc --chapter N --latest-only --extra-credit` (entire extras set).
  - For N=19: each of `--fold-constants`, `--eliminate-unreachable-code`, `--propagate-copies`, `--eliminate-dead-stores` AND default-all.
  - For N=20: both `--no-coalescing` and (default) coalescing.
  - For N∈{5, 10, 15, 18, 20}: `./tests/test_compiler ./target/release/rustcc --chapter N` (no --latest-only) for full regression.

  **Recommended Agent Profile**: Category `unspecified-high`.

  **Parallelization**: Sequential after W22-T1.

  **Acceptance Criteria**:
  - [ ] All gates above report zero `fail` lines.
  - [ ] Save complete log to `.omo/evidence/task-62-full-regression.txt`.

- [ ] 63. W22-T3: Wave 22 - final commit + cleanup

  **What to do**: Final commit, finalize README/COACHING_LOG.
  - Single commit `docs: chapter-by-chapter implementation complete; verification gates green`.
  - Body includes all 20 chapter commands run with results.

  **Recommended Agent Profile**: Category `quick`.

  **Parallelization**: Sequential final step. Blocked By: W22-T2.

  **Acceptance Criteria**:
  - [ ] Single commit on `main` with all gate commands in body.
  - [ ] `git log --oneline -20` shows the per-chapter commits + final.

---

## Final Verification Wave (MANDATORY - after ALL implementation tasks)

4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing. Rejection or user feedback -> fix -> re-run -> present again -> wait for okay.

- [ ] F1. Plan Compliance Audit - `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read files, run chapter gates, inspect `.omo/evidence/`). For each "Must NOT Have": grep for forbidden patterns (e.g., `enum`, `bitfield`, `long long`, `__attribute__`, the deleted interpreter/bridge paths). Confirm all 20 chapters' gates green and all 7 extras cumulatively tested through chapter 18.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. Code Quality Review - `unspecified-high`
  Run `cargo build --release && cargo test`; both must be clean (zero warnings, all tests pass). Run `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --extra-credit` and `--chapter 20 --latest-only --no-coalescing` and `--chapter 20 --latest-only` to confirm the gate is green at the latest chapter.
  Grep the codebase for: `evaluate_program` (interpreter fingerprint), `SystemAssemblySanitizerOptions` (system-C fingerprint), `compile_with_system_cc_frontend` (system-C fingerprint). Any non-zero grep match => REJECT.
  Inspect `src/ir/control_flow.rs` (must NOT exist) and `src/support/source.rs` (must be removed). Compare against OCaml-mirror structure. Flag any variance.
  Output: `Build [PASS/FAIL] | cargo test [PASS/FAIL] | All-chapter-gate [N/20] | Must-NOT fingerprints [CLEAN/N hits] | VERDICT`

- [ ] F3. Real Manual QA - `unspecified-high`
  Run the full pipeline end-to-end on a curated set of `.c` programs spanning all 20 chapters and all 7 extras. For each, compile with `rustcc`, then `gcc -c` the `.s`, then run the resulting executable and confirm the exit code and stdout. Save to `.omo/evidence/final-qa/`. Specifically test: chapter 9 multi-function arg passing, chapter 13 NaN comparison, chapter 14 pointer aliasing, chapter 15 array-decayed arg, chapter 16 string literal in `.rodata`, chapter 17 `sizeof` evaluation, chapter 18 struct passed by value, chapter 19 optimization actually reduces work, chapter 20 program with many temporaries to force register allocation + spill behavior.
  Output: `Scenarios [N/N] | Compilations [N/N PASS] | Executions [N/N correct] | VERDICT`

- [ ] F4. Scope Fidelity Check - `deep`
  Read each task's "Must NOT do" and verify the diff contains only permitted changes. For chapter tasks, verify per-chapter features match `nqcc2/lib/parse.ml` and `nqcc2/lib/semantic_analysis/*.ml`. Confirm no chapter adds features the book does not. Confirm wave 22 finishes with no compiler-stage Rust unit tests beyond the 9 existing.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

Per-chapter commits. Each chapter-gate task green preceded by:
1. A `feat(compiler): chapter N: <feature>` commit on `main`.
2. Test-rerun command in commit body.

Extras (bitwise/compound/increment/goto/switch/nan/union) introduced with chapter: use `feat(extra):` instead.

Wave 0 foundation cleanup is `refactor(compiler): mirror nqcc2 module structure; tear out interpreter and system-C bridge`.

Final Wave 22 commit is `docs: chapter-by-chapter implementation complete; verification gates green` with all per-chapter invocations reproduced in the body.

---

## Success Criteria

### Verification Commands

```bash
# Per-chapter gate (run after each chapter's tasks)
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter N --latest-only [--extras per docs/book/test-map.md]

# Final regression at end of project (Wave 22 + Final Verification)
for N in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18; do
  ./tests/test_compiler ./target/release/rustcc --chapter $N --latest-only --extra-credit
done
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only

cargo test
```

### Final Checklist

- [ ] All 20 chapter gates pass with chapter's extras per `docs/book/test-map.md`.
- [ ] All 7 extras implemented and tested cumulatively from introduction through chapter 18.
- [ ] Chapter 19's 4 optimization passes each pass individually AND the default-all-optimizations run.
- [ ] Chapter 20 passes both `--no-coalescing` and the default-with-coalescing.
- [ ] No "Must NOT Have" patterns (search confirms clean).
- [ ] `cargo build --release` produces zero warnings.
- [ ] `cargo test` passes (9 unit tests).
- [ ] Full regression: `--chapter N` (no `--latest-only`) for `N in {5, 10, 15, 18, 20}` is all-green.
- [ ] Architectural sketch fidelity: every `nqcc2/lib/*.ml` has a corresponding Rust module under `src/`.
- [ ] No `src/ir/control_flow.rs` interpreter, no `src/codegen/emit.rs` system-C sanitizer, no `src/support/source.rs` heuristics remain.
- [ ] `docs/COACHING_LOG.md` records each chapter's gate as having been actually run.

---

## References

External references the executor may consult:

- **Book**: `/home/mei/projects/rustcc/docs/Writing a C Compiler - Sandler, Nora.pdf`
- **OCaml reference (source of truth)**: `/home/mei/projects/rustcc/nqcc2/lib/` (lex.ml, parse.ml, ast.ml, tacky.ml, tacky_gen.ml, semantic_analysis/resolve.ml, semantic_analysis/label_loops.ml, semantic_analysis/typecheck.ml, assembly.ml, cfg.ml, const.ml, emit.ml, backward_dataflow.ml, compile.ml, type_table.ml, const_convert.ml, util/disjoint_sets.ml in `nqcc2/lib/util/`, backend/codegen.ml, backend/instruction_fixup.ml, backend/replace_pseudos.ml, backend/regalloc.ml, backend/assembly_symbols.ml in `nqcc2/lib/backend/`, optimizations/optimize.ml, optimizations/optimize_utils.ml, optimizations/constant_folding.ml, optimizations/copy_prop.ml, optimizations/dead_store_elim.ml, optimizations/unreachable_code_elim.ml, optimizations/address_taken.ml in `nqcc2/lib/optimizations/`)
- **OCaml driver**: `/home/mei/projects/rustcc/nqcc2/bin/main.ml`
- **Chapter guides (Rust-oriented)**: `/home/mei/projects/rustcc/docs/book/ch01-minimal-compiler.md` through `ch20-register-allocation.md`
- **Stage pseudocode**: `/home/mei/projects/rustcc/docs/stages/ch01-minimal-compiler.md` through `ch20-register-allocation.md`
- **Test map**: `/home/mei/projects/rustcc/docs/book/test-map.md`
- **Test runner**: `/home/mei/projects/rustcc/tests/test_compiler` + `/home/mei/projects/rustcc/tests/test_framework/runner.py`
- **Test configuration**: `/home/mei/projects/rustcc/tests/test_properties.json`
