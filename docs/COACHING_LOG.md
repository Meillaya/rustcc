# rustcc Coaching Log

This file is the single running record for the guided build of the compiler.
It collects the current spec, instructions, milestones, and explanations as we go.

---

## Session: 2026-04-15

### Working mode
- Coaching-first.
- Do not do the compiler implementation for the user.
- Provide architecture, sequencing, milestones, and review feedback.
- Keep guidance concrete and incremental.

### Current decision
Develop the **compile driver first**, before the lexer/parser, following the book's recommendation.

### Why start with the compile driver?
- It establishes the outer compilation pipeline early.
- It separates orchestration from compiler internals.
- It lets later lexer/parser/codegen work plug into an existing flow.
- It gives early end-to-end wins even before the compiler internals are real.

### Driver responsibilities
The compile driver is responsible for:
- accepting the input `.c` file
- parsing stage-selection flags
- validating filenames and extensions
- deriving intermediate/output paths
- invoking the system preprocessor
- calling the compiler boundary module
- optionally invoking assembler/linker
- stopping early at requested stages

The compile driver is **not** responsible for:
- tokenization details
- parsing rules
- AST design internals
- code generation internals

### Recommended module boundaries
- `main.rs`: entry point only; parse args, build config, call driver, print errors
- `driver.rs`: orchestration of the compilation pipeline
- `toolchain.rs`: wrappers around external tools like `cc -E -P` and final assemble/link commands
- `compiler.rs`: public boundary for the user's compiler logic; stub now, real internals later

Later internal compiler modules:
- `token.rs`
- `lexer.rs`
- `ast.rs`
- `parser.rs`
- `codegen.rs`

### Boundary rule
- `driver` works with **files, flags, and subprocesses**
- `compiler` works with **source text and compiler data structures**

### Suggested stage model
Use an enum-like mental model for stages:
- preprocess
- lex
- parse
- codegen
- full

Even if some stages are still stubbed, designing around them now will keep the pipeline clean.

### First milestone
Make this work:

`rustcc input.c`
- validate the input file
- preprocess `input.c` into `input.i`
- read the preprocessed text
- pass it into a placeholder compiler entry point
- print a deterministic success result

### Second milestone
Replace the placeholder compiler with a temporary stub that emits fixed assembly for a trivial program such as:

```c
int main(void) { return 2; }
```

Then have the driver:
- write `input.s`
- invoke the system toolchain
- produce an executable

This validates the full shell around the future compiler implementation.

### Current next task
Design `driver::run` as a plain-English control-flow checklist before writing code.

### Control-flow outline for `driver::run`
1. Receive a validated `Config` from `main`.
2. Extract the input path and requested stage.
3. Confirm the input exists and has a `.c` extension.
4. Derive intermediate/output paths:
   - `.i` for preprocessed source
   - `.s` for assembly
   - executable path for full builds
5. Invoke the preprocessor through `toolchain`.
6. If the requested stage is preprocess-only, stop successfully here.
7. Read the generated `.i` file into a string.
8. Pass that string into the public function in `compiler.rs`.
9. Depending on the selected stage:
   - print tokens
   - print AST
   - write assembly
   - or continue toward a final executable
10. If assembly was produced and the stage is full, call the toolchain again to assemble/link.
11. Return success, or propagate a structured error.

### Design guidance for `driver::run`
- Keep it linear and easy to trace.
- Prefer one stage boundary per block.
- Make early-return points explicit for stop-stage flags.
- Keep external command details inside `toolchain.rs`.
- Keep compiler internals behind one public API in `compiler.rs`.

### Coaching note
Do not over-generalize the driver on day one. The goal is a clean, testable path through the pipeline, not a perfect abstraction.

### Scaffold created
Created the initial compile-driver source layout:
- `src/main.rs`
- `src/driver.rs`
- `src/toolchain.rs`
- `src/compiler.rs`

Current scaffold behavior:
- the binary builds
- the driver accepts a simple input path plus stage flags
- the driver validates `.c` input
- the driver derives `.i`, `.s`, and executable paths
- the driver prints the planned pipeline instead of executing it

This is intentional: the project now has the correct outer shape without jumping ahead into the real implementation.

### Immediate next implementation target
Replace the scaffold print-only path with the first real pipeline action:
1. implement preprocessing in `toolchain.rs`
2. call it from `driver::run`
3. stop successfully at preprocess-only mode
4. read the generated `.i` file
5. send that text into the placeholder compiler boundary
Planning placeholder directories created only for plan artifact targeting; no implementation files written yet.

## Session update: 2026-04-15 ralplan artifacts written
- Consensus plan approved via planner → architect → critic loop.
- Wrote .omx/plans/prd-rustcc-book-package.md.
- Wrote .omx/plans/test-spec-rustcc-book-package.md.
- Tightened docs naming and scaffold banlist policy in the consensus plan.

## Session update: 2026-04-16 ralph execution complete
- Authored full docs/book package (20 chapter guides + backbone + maps + appendices + templates).
- Authored docs/specs SRS package.
- Authored docs/research resource package, including blogs-and-papers.
- Moved the placeholder scaffold into `src/` at the user's request.
- Wrote verification and deslop reports under .omx/plans/.

## Wave 0 Verification — Foundation Rewrite

**Date**: 2026-07-07T19:48:15Z

**Gate commands**:
- `cargo build --release` → exit 0, zero warnings
- `cargo test --release` → 9 passed, 0 failed
- Fingerprint greps (all zero matches in `src/`):
  - `evaluate_program`, `compile_with_system_cc_frontend`, `source_has_*`, `should_defer_parse_to_system_frontend`, `semantic_error_that_should_parse`, `likely_parse_error`, `likely_struct_or_union_parse_error`
  - `evaluate_with_system_cc`, `system_c_syntax_check`, `system_c_to_assembly`, `write_temp_c_source` in `src/toolchain.rs`
  - `sanitize_system_assembly`, `SystemAssemblySanitizerOptions`

**Deleted files**:
- `src/ir/control_flow.rs` → `No such file or directory` (interpreter removed)
- `src/support/source.rs` → `No such file or directory` (heuristic gate removed)
- `src/codegen/emit.rs` → path now holds the new OCaml-mirror codegen emitter (`emit()` pretty-prints `AsmProgram` to x86-64 AT&T text). The old system-C sanitizer content is gone; verified by zero matches for `sanitize_system_assembly` / `SystemAssemblySanitizerOptions`.

**OCaml-mirror layout**:
- `src/semantics/{resolve,label_loops,typecheck}.rs`
- `src/ir/{tacky,lower,opt,cfg,temp}.rs`
- `src/codegen/{assembly,assembly_symbols,abi,codegen,emit,fixup,frame,regalloc,replace_pseudos}.rs`

**Kept gcc helpers** in `src/toolchain.rs`:
- `preprocess()` for `gcc -E -P`
- `assemble_only()` / `assemble_and_link()` for final gcc invocation

**Evidence**: `/home/mei/projects/rustcc/.omo/evidence/task-7-wave0-gate.txt`
