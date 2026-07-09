# Task 50 Chapter 18 Aggregate ABI Code Review

Verdict: **BLOCK**

Reviewed current task-50 working tree in `/home/mei/projects/rustcc` against `HEAD`, focused on aggregate ABI changes and restored Chapter 18 Linux assembly fixtures.

## Evidence inspected

- Required implementation note: `.omo/evidence/task-50-ch18-abi-implementation.txt`
- Diff since `HEAD`: `src/codegen/abi.rs`, `src/codegen/codegen.rs`, `src/codegen/assembly.rs`, `src/codegen/emit.rs`, `src/codegen/replace_pseudos.rs`, `src/ir/lower.rs`, `src/ir/tacky.rs`
- Restored fixture paths under `tests/tests/chapter_18/**/*.s`
- `git diff --check HEAD`: pass
- `cargo check`: pass
- LSP diagnostics on all modified Rust files: no diagnostics found
- ast-grep focused patterns: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` matches in the focused modified source. Existing `unreachable!` sites in `src/ir/lower.rs` were not added by this diff.

## Findings by severity

### HIGH

1. **Restored Linux `.s` fixtures are ignored and not part of the commit diff**

   Files:
   - `.gitignore:17` (`*.s` ignores the restored fixtures)
   - `tests/tests/chapter_18/valid/parameters/data_on_page_boundary_linux.s`
   - `tests/tests/chapter_18/valid/params_and_returns/data_on_page_boundary_linux.s`
   - `tests/tests/chapter_18/valid/params_and_returns/big_data_on_page_boundary_linux.s`
   - `tests/tests/chapter_18/valid/params_and_returns/validate_return_pointer_linux.s`
   - `tests/tests/chapter_18/valid/params_and_returns/return_space_address_overlap_linux.s`

   Evidence: `git check-ignore -v` reports all five fixtures ignored by `.gitignore:17`, and `git status --short --ignored` reports them as `!!`. `git diff --name-only HEAD` does not include them.

   Risk: Task 50 evidence depends on these fixtures for Chapter 18 page-boundary and hidden-return tests. A commit containing only the tracked Rust diffs will omit the fixtures, so a clean checkout can regress to the same missing-fixture errors described in the implementation evidence.

   Minimal fix: force-add the five intended fixture files (`git add -f <paths>`) or add narrow `.gitignore` exceptions for these specific Chapter 18 Linux fixtures, then include them in the task-50 commit. Do not stage as part of this review; this is the required follow-up before merge/gate closure.

### MEDIUM

None blocking in source code.

### LOW / WATCH

1. **Broad rvalue fallback should remain intentional**

   File: `src/ir/lower.rs:1769-1778`

   Assessment: `lower_aggregate_source_pointer` catches any `Err(_)` from `lower_lvalue_address` and falls back to `lower_expr`. This appears intentional for lvalue-vs-rvalue aggregate handling and does not silently drop IR: it either obtains an address or materializes the expression and addresses the resulting temporary. However, it is broad enough to mask future addressability errors if `lower_lvalue_address` grows new failure modes.

   Suggested hardening: if this area changes again, introduce a typed/non-lvalue error path and fall back only for that case.

## Required checks

### No-bridge / no-test-weakening

- Source-content bridge: **PASS**. No added source reads, fixture-name dispatch, `include_str!`, `read_to_string`, `std::fs` fallback, frontend/OCaml bridge, or hard-coded Chapter 18 test names in the task diff.
- Test/harness weakening: **PASS** for tracked diffs. No tracked test harness changes were present in `git diff --name-only HEAD -- tests ...`.
- Caveat: restored `.s` fixtures exist on disk but are ignored/untracked; this is the blocking issue above, not a harness weakening.

### Unsafe / panic / debug leftovers

- `unsafe`: **PASS**. No additions and no focused-source matches.
- `unwrap` / `expect` / `panic` additions: **PASS**. No additions detected.
- `todo` / `unimplemented` / `dbg` / `println` / `eprintln`: **PASS**. No debug leftovers detected in the task diff.
- Existing `unreachable!` sites in `src/ir/lower.rs` are not newly introduced by task 50.

### Silent catch-all IR drops

- **PASS with LOW watch**. The new `Err(_)` fallback in `lower_aggregate_source_pointer` does not drop instructions; it falls back from address-of-lvalue lowering to expression materialization. No added `_ => Vec::new()` or equivalent silent drop was found in the diff.

### Exact byte-copy / page-boundary behavior

- **PASS from code inspection**. Aggregate copy paths use exact slot sizes for partial eightbytes:
  - `abi::eightbyte_size` clamps each eightbyte slot to remaining bytes.
  - `copy_mem_to_reg`, `copy_reg_to_mem`, `copy_mem_to_stack`, and `copy_bytes_to_address` use byte loops for partial slots rather than over-reading to a full eightbyte.
  - Hidden-memory returns copy full 8-byte chunks only while `offset + 8 <= size`, then byte-copy the tail.

### Hidden return pointer correctness

- **PASS from code inspection**. Large aggregate returns consume `%rdi`, reduce integer parameter capacity, save the hidden pointer in the callee prologue, copy the returned aggregate into that address, and return the hidden pointer in `%rax`.
- The restored `validate_return_pointer_linux.s` fixture specifically validates `%rax` after `return_in_mem`, but it must be tracked before this protection is reproducible.

### `PseudoMem` / `DataOffset` design consistency

- **PASS**. `Operand::PseudoMem` and `Operand::DataOffset` are consistently represented across assembly AST, emitter formatting, pseudo replacement, and memory-to-memory fixups.
- `replace_pseudos` maps local `PseudoMem` to stack offsets and global `PseudoMem` to `DataOffset`; emitter rejects leaked pseudo operands.

### Restored fixture legitimacy

- **Content legitimacy: PASS**. The five restored fixtures are plausible and match the C tests' intent:
  - page-boundary objects use `.balign 4096` plus `.skip` so 11-, 10-, and 18-byte objects end on a page boundary;
  - `validate_return_pointer_linux.s` calls `return_in_mem` and validates result fields through `%rax`;
  - `return_space_address_overlap_linux.s` rejects overlap with `globvar` or the input pointer and writes expected return values.
- **Version-control legitimacy: BLOCK**. The fixtures are ignored/untracked and absent from `git diff`, so they are not yet commit-safe.

### ABI scope fidelity

- **PASS from focused source review**. The implementation is scoped to Chapter 18 aggregate ABI behavior: aggregate classification, parameter/return register vs stack planning, hidden return pointer handling, aggregate rvalue materialization, and byte-copy support. No source bridge, hard-coded test-name behavior, or test weakening was found.

## Task 50 blocking status

**BLOCKS task 50:** yes. The only blocking issue found is that the restored Linux `.s` fixtures required by the green Chapter 18 evidence are ignored/untracked and would be omitted from a normal commit.
