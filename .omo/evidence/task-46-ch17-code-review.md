# Task 46 Chapter 17 Code Quality Re-review

Date: 2026-07-08
Repository: `/home/mei/projects/rustcc`
Review mode: read-only code-quality re-review; implementation files were not edited.

## Verdict

PASS / APPROVE with WATCH notes.

The previous Chapter 17 blockers are fixed: the named invalid tests and the block-scope void-array parameter probe now fail during `--validate`, and the Chapter 17 latest-only harness is green when run sequentially.

## Skill-perspective check

- `omo:remove-ai-slops`: consulted by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md` before judging tests/production code. Result: no deletion-only tests, no tautological fake-pass tests, no harness/plan edits, no bridge fallback, and no needless production parsing/extraction outside the Chapter 17 grammar/typechecking surface. WATCH: the in-source `sizeof` test is debug-output-coupled but not useless because it checks that the operand side effect is not lowered.
- `omo:programming`: consulted by reading `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`, `references/rust/README.md`, and `references/code-smells.md`. Result: no new unsafe, no untyped escape hatch, no new dependency, no LSP diagnostics on changed files. WATCH: several touched compiler modules remain far above the 250 pure-LOC programming review ceiling, but this is pre-existing architecture in the active compiler passes and not a fake-pass or scope-drift blocker for the focused fixes.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

The previous HIGH findings are resolved:

1. `not_void` / `!void` rejection
   - Code: `src/semantics/typecheck.rs:411-417` now rejects `UnaryOp::Not` when the operand type is `Void`.
   - Evidence: `./target/release/rustcc --validate tests/tests/chapter_17/invalid_types/scalar_expressions/not_void.c` exits `1` with `type error: logical not requires scalar operand`.

2. `return_void_as_pointer` rejection
   - Code: `src/semantics/typecheck.rs:638-643` no longer treats casts as transparent in `is_null_pointer_constant`, so `(void)0` is not assignable as a null pointer constant.
   - Evidence: `./target/release/rustcc --validate tests/tests/chapter_17/invalid_types/void/return_void_as_pointer.c` exits `1` with `type error: return expression has incompatible type`.

3. `void_equality` rejection
   - Code: `src/semantics/typecheck.rs:646-653` rejects equal `Void` operands before returning success for same-type comparisons.
   - Evidence: `./target/release/rustcc --validate tests/tests/chapter_17/invalid_types/void/void_equality.c` exits `1` with `type error: cannot compare void expressions`.

4. Block-scope void-array parameter validation
   - Code: `src/semantics/typecheck.rs:139-147` routes block-scope function declarations through `validate_function_signature`; `src/semantics/typecheck.rs:77-89` validates all parameter and return types; `src/semantics/typecheck.rs:755-772` recursively rejects arrays whose element type is `void`, including under pointers.
   - Evidence: direct block-scope probe `int main(void) { int foo(void (*bad_array)[3]); return 0; }` exits `1` with `type error: object has void type`.
   - Evidence: `./target/release/rustcc --validate tests/tests/chapter_17/invalid_types/incomplete_types/void_array_pointer_in_param_type.c` exits `1` with `type error: object has void type`.

### MEDIUM

1. `cargo clippy --release --all-targets -- -D warnings` is not clean.
   - Status: WATCH, not a blocker for this focused re-review because the repo has broad pre-existing Clippy debt and the requested verification gate was `cargo check` plus Chapter 17 behavior.
   - Changed/relevant examples include `src/ast/expr.rs:35` (`SizeOfExpr` trips `clippy::enum_variant_names`) and collapsible conditionals in `src/semantics/typecheck.rs:90`, `100`, `155`, and `669`.

2. The `sizeof` in-source unit test is brittle/debug-format-coupled.
   - Code: `src/compiler.rs:210-219` asserts exact TACKY pretty-output fragments and a synthetic temporary name.
   - It is not useless or tautological: it would catch accidental lowering/evaluation of `sizeof(x = 2)`. However, the remove-ai-slops/programming perspective would prefer a structured IR assertion or observable behavior over pretty-string fragments.

3. Touched modules remain oversized.
   - Pure LOC measured after fixes: `src/ir/lower.rs` 2015, `src/parse/parser.rs` 1098, `src/semantics/typecheck.rs` 732, `src/semantics/resolve.rs` 657, `src/semantics/label_loops.rs` 285.
   - This remains a maintainability risk, but the additions are in the expected parser/typechecker/lowerer seams for Chapter 17 and are not a bridge/fallback or scope-drift mechanism.

### LOW

1. Some comments are stale relative to the expanded Chapter 17 surface.
   - Example: `src/parse/parser.rs:647-651` still describes parameter lists as `(void)` or `int` parameters even though the parser now handles broader Chapter 17 types.
   - Non-blocking documentation/style issue.

## Verification performed

- `git diff --check`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo check`: PASS, `Finished dev profile`.
- `cargo build --release`: PASS, `Finished release profile`.
- LSP diagnostics: first daemon request timed out while auto-starting; retry on each changed file reported no diagnostics for:
  - `src/ast/expr.rs`
  - `src/ast/stmt.rs`
  - `src/ast/ty.rs`
  - `src/compiler.rs`
  - `src/ir/lower.rs`
  - `src/parse/parser.rs`
  - `src/semantics/label_loops.rs`
  - `src/semantics/resolve.rs`
  - `src/semantics/typecheck.rs`
- `cargo test --release`: PASS, 10 tests.
- Direct invalid validate probes: PASS, all rejected:
  - `invalid_types/scalar_expressions/not_void.c`: exit `1`, `type error: logical not requires scalar operand`.
  - `invalid_types/void/return_void_as_pointer.c`: exit `1`, `type error: return expression has incompatible type`.
  - `invalid_types/void/void_equality.c`: exit `1`, `type error: cannot compare void expressions`.
  - `invalid_types/incomplete_types/void_array_pointer_in_param_type.c`: exit `1`, `type error: object has void type`.
  - Scratch block-scope declaration `int main(void) { int foo(void (*bad_array)[3]); return 0; }`: exit `1`, `type error: object has void type`.
- `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only --stage validate`: PASS, 70 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only`: PASS, 70 tests OK.
  - Note: an initial parallel run of validate-stage and full latest-only collided on shared fixture-derived intermediate files and produced missing `.i`/`.s` errors. The sequential reruns above are the trusted results.
- Manual CLI smoke probes:
  - `int main(void) { return sizeof(int); }`: compile exit `0`, program exit `4`.
  - `int main(void) { void *p = (void *)0; return p == 0; }`: compile exit `0`, program exit `1`.
  - `--tacky` for `sizeof(x = 2)` emitted `Constant(4)`/`ULong` and no `Constant(2)` assignment lowering.
- `cargo clippy --release --all-targets -- -D warnings`: FAIL with existing broad lint debt plus changed-line style warnings; recorded under MEDIUM/WATCH, not a focused Chapter 17 blocker.

## Scope-control checks

- Changed tracked files are only:
  - `src/ast/expr.rs`
  - `src/ast/stmt.rs`
  - `src/ast/ty.rs`
  - `src/compiler.rs`
  - `src/ir/lower.rs`
  - `src/parse/parser.rs`
  - `src/semantics/label_loops.rs`
  - `src/semantics/resolve.rs`
  - `src/semantics/typecheck.rs`
- No tracked tests, harnesses, docs, or plan files changed.
- `git diff --cached --name-only`: no staged files.
- `.omo/plans/c-compiler-rust.md:1693` remains unchecked for task 46; no plan checkbox diff exists.
- Forbidden bridge/source-fallback fingerprint scan in `src/`: no matches for `evaluate_program`, `compile_with_system_cc_frontend`, `SystemAssemblySanitizerOptions`, `sanitize_system_assembly`, `should_defer_parse_to_system_frontend`, `system_c_to_assembly`, `system_c_syntax_check`, `evaluate_with_system_cc`, `gcc_array_subset_assembly`, `source.contains`, `source_has_`, `likely_parse_error`, `semantic_error_that_should_parse`, `bridge fallback`, or `fallback`.

## Recommendation

APPROVE. No CRITICAL or HIGH blockers remain.

## Blockers

None.
