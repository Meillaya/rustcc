# Task 50 Chapter 18 Code Review 3 - Stale ABI Scaffolding Cleanup

Verdict: PASS
Date: 2026-07-08
Scope: read-only re-review after stale ABI scaffolding cleanup, focused on `src/codegen/abi.rs` and `src/codegen/codegen.rs`; only this evidence file was written.

## Required startup marker

`WORKING: task 50 code review 3 - stale scaffolding cleanup`

## Stage 1 - Spec / gate compliance

PASS. The cleanup addresses the prior gate rejection:

- `src/codegen/abi.rs` no longer contains the scalar-era `ParamClass`, `AbiPlan`, `classify_params`, or `XMM_PARAM_REGS` API. Live ABI placement is represented by `ClassifiedParams` with separate `int_slots`, `sse_slots`, and `stack_slots`.
- `src/codegen/abi.rs` `ParamSlot` now contains only `param_index`, `offset`, and `size`; there is no redundant `PassingClass` enum or `ParamSlot.class` field.
- `src/codegen/codegen.rs` `CodegenCtx` contains `function_param_types`, `function_return_types`, `current_return_on_stack`, and `current_function_name`; there is no unused `current_return_type` field.
- `.omo/evidence/task-50-ch18-abi-implementation.txt` tail records the cleanup and full post-cleanup gates as green: LSP diagnostics, fmt check, release build, cargo test, chapter 18 latest-only, chapter 18 latest-only `--union`, chapter 17 latest-only, chapter 16 latest-only, bridge scan, stale scaffolding scan, and `git diff --check`.

## Required scans / evidence

### Stale symbol scan

Command:

```sh
rg -n "\b(ParamClass|AbiPlan|classify_params|PassingClass|current_return_type|XMM_PARAM_REGS)\b" src/codegen src/ir || true
```

Result: no output. The stale symbols are gone from live source under `src/codegen` and `src/ir`.

A broader historical scan did find old mentions in prior `.omo/evidence/*` files and the legitimate C fixture names `tests/tests/chapter_18/valid/parameters/libraries/classify_params*.c`; those are historical/test-program names, not live Rust ABI scaffolding.

### Focused file inspection

- `src/codegen/abi.rs:12-23`: `ParamSlot` has only `param_index`, `offset`, `size`; `ClassifiedParams` partitions slots by passing location.
- `src/codegen/abi.rs:125-209`: `classify_typed_parameters` is the current parameter classifier and emits directly into the three slot vectors.
- `src/codegen/codegen.rs:190-198`: `CodegenCtx` has no `current_return_type`.
- `src/codegen/codegen.rs:414-425` and `src/codegen/codegen.rs:1922-1933`: call lowering and function prologue use `function_return_types`, `current_return_on_stack`, and `classify_typed_parameters`.

### Fixture force-add requirement and no harness weakening

Commands:

```sh
git diff --name-status -- tests tests/test_compiler tests/test_framework tests/test_properties.json tests/expected_results.json
git check-ignore -v \
  tests/tests/chapter_18/valid/parameters/data_on_page_boundary_linux.s \
  tests/tests/chapter_18/valid/params_and_returns/data_on_page_boundary_linux.s \
  tests/tests/chapter_18/valid/params_and_returns/big_data_on_page_boundary_linux.s \
  tests/tests/chapter_18/valid/params_and_returns/validate_return_pointer_linux.s \
  tests/tests/chapter_18/valid/params_and_returns/return_space_address_overlap_linux.s
git status --short --ignored=matching -- <same five fixture paths>
git ls-files --stage -- <same five fixture paths>
```

Results:

- Tracked tests/harness diff: no output. No tracked test/harness weakening was found.
- All five Chapter 18 Linux `.s` fixtures exist on disk.
- `git check-ignore -v` reports `.gitignore:17:*.s` for each of the five fixtures.
- `git status --short --ignored=matching` reports all five fixtures as `!!` ignored.
- `git ls-files --stage` reports no rows for the five fixtures.

Conclusion: the fixture state is unchanged from code-review-2. The exact five fixtures remain an explicit commit-lane `git add -f` requirement, not a product-code blocker for this stale-scaffolding cleanup re-review.

### Anti-pattern / bridge scans

Commands:

```sh
git diff --unified=0 -- src/codegen/abi.rs src/codegen/codegen.rs | rg -n '^\+.*(unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|dbg!|println!|eprintln!|std::fs|read_to_string|include_str!|Command::new\("(clang|nqcc|ocaml)|OCAMLRUN|frontend fallback|source bridge|fallback.*frontend)' || true
sg -p 'println!($$$ARGS)' -l rust src/codegen/abi.rs src/codegen/codegen.rs || true
sg -p 'dbg!($$$ARGS)' -l rust src/codegen/abi.rs src/codegen/codegen.rs || true
sg -p 'panic!($$$ARGS)' -l rust src/codegen/abi.rs src/codegen/codegen.rs || true
sg -p 'unsafe { $$$BODY }' -l rust src/codegen/abi.rs src/codegen/codegen.rs || true
sg -p 'unwrap()' -l rust src/codegen/abi.rs src/codegen/codegen.rs || true
```

Results: no findings in focused files/diff.

## Diagnostics / checks run this review

- `mcp__lsp.diagnostics` on every modified Rust file:
  - `src/codegen/abi.rs`: no diagnostics
  - `src/codegen/codegen.rs`: no diagnostics
  - `src/codegen/assembly.rs`: no diagnostics
  - `src/codegen/emit.rs`: no diagnostics
  - `src/codegen/replace_pseudos.rs`: no diagnostics
  - `src/ir/lower.rs`: no diagnostics
  - `src/ir/tacky.rs`: no diagnostics
- `cargo check --quiet`: pass
- `cargo fmt --all -- --check`: pass
- `git diff --check`: pass

## Issues

No CRITICAL, HIGH, MEDIUM, or LOW issues found in the stale-scaffolding cleanup re-review.

## Recommendation

PASS / APPROVE.

No new blockers were found. The prior stale ABI scaffolding blocker is resolved, focused source diagnostics are clean, tracked tests/harness remain unchanged, and the existing fixture force-add requirement remains explicit for the commit lane.
