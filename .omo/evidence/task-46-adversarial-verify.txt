Task 46 adversarial verification: W18-T1 Chapter 17 void/sizeof/dynamic-memory support
Date: 2026-07-08
Workspace: /home/mei/projects/rustcc
Recommendation: APPROVE

originalIntent:
- Implement Chapter 17 support in the native Rust compiler pipeline: Type::Void, void/void* rules, sizeof(expr)/sizeof(type), return; for void functions, and external dynamic-memory declarations/calls without any system-C frontend/bridge fallback.

desiredOutcome:
- Current worktree passes the real required release build/test/chapter gates.
- Manual user-visible examples work: sizeof(int) exits 4 and void* null comparison exits 1.
- sizeof operands are type-checked but not lowered/evaluated for side effects.
- No tracked test/harness files are weakened or modified.

checkedArtifactPaths:
- .omo/plans/c-compiler-rust.md (task 46 lines 1693-1723; remains unchecked)
- .omo/evidence/task-46-ch17-implementation.txt
- .omo/evidence/task-46-ch17-code-review.md
- current git diff for src/ast/expr.rs, src/ast/stmt.rs, src/ast/ty.rs, src/compiler.rs, src/ir/lower.rs, src/parse/parser.rs, src/semantics/label_loops.rs, src/semantics/resolve.rs, src/semantics/typecheck.rs

userOutcomeReview:
- Task 46's requested behaviors are present in current product source and pass the required executable gates.
- Tests/harness were not modified: `git status --porcelain=v1 -- tests` produced no output; `git diff -- tests` and `git diff --cached -- tests` produced no output.
- Worktree is intentionally dirty for product source only; no staged files.

commandEvidence:
- `cargo build --release` => exit 0; `Finished release profile [optimized] target(s) in 0.05s`.
- `cargo test --release` => exit 0; 10 bin tests passed, 0 failed; doc-tests 0 passed.
- `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only` => exit 0; `Ran 70 tests in 3.327s`, `OK`.
- `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` => exit 0; `Ran 72 tests in 1.325s`, `OK` (assembler warning on explicit_casts.s, gate still PASS).
- `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only` => exit 0; `Ran 83 tests in 1.455s`, `OK`.
- Forbidden bridge scan command:
  `rg -n 'gcc_array_subset_assembly|source\.contains\("\[|-std=c17|system_c_to_assembly|compile_with_system_cc_frontend|sanitize_system_assembly|evaluate_with_system_cc|system_c_syntax_check|should_defer_parse_to_system_frontend|source_has_|likely_parse_error|semantic_error_that_should_parse|evaluate_program|SystemAssemblySanitizerOptions|bridge fallback|fallback' src`
  => rg exit 1 normalized to PASS; no matches.
- Manual acceptance 1:
  source `int main(void) { return sizeof(int); }`
  `./target/release/rustcc /tmp/task46_gate_sizeof.c` => compile exit 0; `/tmp/task46_gate_sizeof` => program exit 4.
- Manual acceptance 2:
  source `int main(void) { void *p = (void *)0; return p == 0; }`
  `./target/release/rustcc /tmp/task46_gate_voidptr.c` => compile exit 0; `/tmp/task46_gate_voidptr` => program exit 1.
- Dynamic memory external declaration/call smoke:
  source declares `extern void *malloc(unsigned long size); extern void free(void *ptr);`
  `./target/release/rustcc /tmp/task46_gate_malloc_free.c` => compile exit 0; program exit 1.
- Native stage non-evaluation probe:
  source `int main(void) { int x = 1; return sizeof(x = 2); }`
  `./target/release/rustcc --tacky /tmp/task46_gate_sizeof_side_effect.c > /tmp/task46_gate_sizeof_side_effect.tacky` => exit 0.
  TACKY body contains `Constant(1)` copied to `main.x.0`, `Constant(4)` copied to `const.0`, `Return(tmp.0)`, type_env `const.0: ULong`; probe script reported `contains_const_2=False`, `contains_store_of_2_to_x=False`, constants seen `['1,', '4,']`.
- Malformed/incomplete type probes:
  `./target/release/rustcc --validate /tmp/task46_gate_sizeof_void.c` for `sizeof(void)` => exit 1, `type error: cannot apply sizeof to incomplete type`.
  `./target/release/rustcc --validate /tmp/task46_gate_void_object.c` for `void x;` => exit 1, `type error: object has void type`.
  `./target/release/rustcc --validate /tmp/task46_gate_voidptr_arith.c` for `void *p; p = p + 1;` => exit 1, `type error: pointer arithmetic requires complete pointed-to type`.
- Stale-state probe:
  `cargo build --release -vv` after gates reported `Fresh rustcc v0.0.1`; target binary mtime 2026-07-08T16:50:02.894787, latest changed Rust source mtime 2026-07-08T16:49:44.495747, binary sha256 `b15b058c4cc18e8d8ea058b4f4b3d4f9aea37e78c783b495d3342eb08549241c`.
- Dirty-worktree probe:
  `git diff --name-status` lists only 9 tracked source files under `src/`; `git diff --cached --name-status` empty; tracked tests/harness status empty.
- Whitespace diff check: `git diff --check` => exit 0.

adversarialClassResults:
- stale_state: PASS; release binary rebuilt/fresh and used for every executable probe.
- dirty_worktree: PASS for gate purposes; dirty tracked files are product source only, no staged changes, no tracked tests/harness modifications.
- misleading_success_output: PASS; verdict uses command exit codes plus outputs, not success prose alone.
- bridge_bypass: PASS; forbidden bridge/fallback fingerprints absent from `src`.
- malformed_input/incomplete_type: PASS; `sizeof(void)`, `void x`, and void-pointer arithmetic reject with nonzero validate status.
- non_evaluation_of_sizeof: PASS; native TACKY has sizeof constant 4 and no assignment-side-effect lowering for `x = 2`.

antiSlopProgrammingReview:
- Loaded/consulted `omo:remove-ai-slops` and `omo:programming` plus Rust/code-smell references before approval.
- Code review report explicitly contains the same skill-perspective coverage and overfit/slop checks.
- Direct pass found no deletion-only tests, no weakened harness/plan edits, no tautological fake-pass tests, no new dependency, no system-C bridge fallback, and no unnecessary production extraction/normalization outside the expected Chapter 17 parser/typechecker/lowerer seams.
- WATCH/non-blocking: `cargo clippy --release --all-targets -- -D warnings` is not clean due broad existing lint debt and some changed-line style warnings (`SizeOfExpr` enum-variant-name per requested plan naming, collapsible ifs). The required task gates, manual QA, and user-visible Chapter 17 behavior are green. Touched compiler modules remain oversized from pre-existing architecture.

blockers:
- None.

evidenceGaps:
- No notepad path was supplied in the verification brief; task-specific required artifacts were present and inspected.
- I did not modify product source/tests and did not add new regression tests during this read-only gate.

finalVerdict: APPROVE
