Adversarial verification report: task 44 verify 2 / Chapter 16 green gate
Date: 2026-07-08
Workspace: /home/mei/projects/rustcc
recommendation: APPROVE

originalIntent:
- Complete task 44 (W17-T1): native Rust compiler support for Chapter 16 characters and string literals.
- Required user-visible outcomes from .omo/plans/c-compiler-rust.md task 44:
  1. `int main(void) { char c = 'A'; return c; }` exits 65.
  2. `int main(void) { char *s = "hello"; return s[0]; }` exits 104.
  3. `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` is green.
- Additional current verification target: after restoring the missing Linux page-boundary assembly fixture and the byte stack-argument fix, prove the fixture-backed page-boundary case no longer segfaults and does not rely on a source-content bridge.

desiredOutcome:
- A user can build the current worktree, run the Chapter 16 latest-only harness, and compile/run representative char/string programs successfully.
- The page-boundary helper fixture is present in the current worktree so the Chapter 16 harness can execute the byte stack-argument boundary test.
- No tests or harness code are weakened, and no native compiler bypass/source-content bridge is used.

userOutcomeReview:
- PASS for the current worktree. The required cargo build/test gates pass, Chapter 16 latest-only is green (`Ran 72 tests ... OK`), Chapter 15 and Chapter 14 latest-only regressions are green, and the bridge scan over `src/` has no matches.
- PASS for manual acceptance. Fresh manual programs returned 65 for `char c = 'A'` and 104 for `"hello"[0]`.
- PASS for the previously failing page-boundary case. The regenerated assembly reads `zed` with a byte-width extension into a register (`movsbl zed(%rip), %r10d`) before `pushq %r10`; manual link/run with `data_on_page_boundary_linux.s` exits 1, not 139/-11.
- PASS for test/harness integrity. `git diff -- tests tests/test_framework tests/test_properties.json` is empty; the local `.s` file is a restored assembly dependency required by tracked test metadata, not a harness weakening.
- Fixture durability warning: the fixture is required and should be force-added with `git add -f tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s` because `.gitignore:17` ignores `*.s` and `git ls-files` currently does not track it. This is not a blocker for the current-worktree approval, but it is required before a durable commit/clean checkout can reproduce the green gate.

blockers:
- None for current worktree approval.

checked artifact paths:
- .omo/plans/c-compiler-rust.md (task 44 section, lines around 1638-1662)
- .omo/evidence/task-44-ch16-implementation.txt
- .omo/evidence/task-44-ch16-code-review.txt
- .omo/evidence/task-44-ch16-gate-review.md
- .omo/evidence/task-44-adversarial-verify.txt
- .omo/evidence/task-44-adversarial-verify-gate-review.md
- .omo/evidence/task-44-ch16-qa/00-preflight.txt through 10-artifact-inventory.txt
- .omx/notepad.md
- tests/test_properties.json
- tests/test_framework/basic.py
- tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c
- tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s
- git diff/status for src, tests, tests/test_framework, tests/test_properties.json, and the fixture path

changed files observed:
- Product Rust files modified in git diff: src/ast/expr.rs, src/ast/ty.rs, src/codegen/assembly.rs, src/codegen/codegen.rs, src/codegen/emit.rs, src/codegen/replace_pseudos.rs, src/compiler.rs, src/ir/lower.rs, src/ir/tacky.rs, src/parse/parser.rs, src/semantics/resolve.rs, src/semantics/typecheck.rs.
- Tests/harness tracked diff: none.
- Fixture file: tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s exists locally but is ignored and untracked.

required command results:
1. `cargo build --release`
   - Exit: 0
   - Evidence: `Finished release profile [optimized] target(s) in 0.03s`.

2. `cargo test --release`
   - Exit: 0
   - Evidence: main tests `10 passed; 0 failed`; doc-tests `0 passed; 0 failed`.

3. `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only`
   - Exit: 0
   - Evidence: `Ran 72 tests in 1.437s` / `OK`.

4. `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only`
   - Exit: 0
   - Evidence: `Ran 83 tests in 1.536s` / `OK`.

5. `./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only`
   - Exit: 0
   - Evidence: `Ran 53 tests in 1.002s` / `OK`.

6. Forbidden bridge scan
   - Command: `rg -n "gcc_array_subset_assembly|source\.contains\(\"\\[|-std=c17|system_c_to_assembly|compile_with_system_cc_frontend|sanitize_system_assembly|evaluate_with_system_cc|system_c_syntax_check|should_defer_parse_to_system_frontend|source_has_|likely_parse_error|semantic_error_that_should_parse" src`
   - Ripgrep exit: 1
   - Evidence: no matches; classified PASS_NO_MATCHES.

manual acceptance examples:
1. `int main(void) { char c = 'A'; return c; }`
   - Compile exit: 0
   - Run exit: 65

2. `int main(void) { char *s = "hello"; return s[0]; }`
   - Compile exit: 0
   - Run exit: 104

page-boundary case:
- Source inspected: `tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c` declares `extern char zed` and passes `zed` as the seventh argument so it is stack-passed.
- Fixture inspected: `tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s` defines global `zed` after `.balign 4096` and `.skip 4095`, placing it at the page boundary.
- `./target/release/rustcc -S tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c` exit: 0.
- Assembly evidence in regenerated `push_arg_on_page_boundary.s`:
  - line 38: `movsbl zed(%rip), %r10d`
  - line 39: `pushq %r10`
  - line 40: `call foo`
- The fixture uses plain/signed `char`, so `movsbl` is the expected byte-extension path for this case. The code diff also contains the `UByte`/`movzbl` stack-argument branch; the page-boundary fixture itself exercises the signed/plain-char path.
- `./target/release/rustcc -c tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c` exit: 0.
- `gcc -D SUPPRESS_WARNINGS ... push_arg_on_page_boundary.o ... data_on_page_boundary_linux.s -o /tmp/rustcc-task44-verify2/manual/push_arg_on_page_boundary` exit: 0.
- Running the linked executable exit: 1. This confirms no segfault (not shell 139 / Python -11) and matches the expected test result.

fixture dependency and force-add decision:
- Required: YES.
- Evidence: `tests/test_properties.json` maps `chapter_16/valid/chars/push_arg_on_page_boundary.c` to `chapter_16/valid/chars/data_on_page_boundary`; `tests/test_framework/basic.py` reads `assembly_libs`, appends the platform suffix, and links the resulting `_linux.s` file with GCC.
- Current ignore/tracking state:
  - `git check-ignore -v tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s` -> `.gitignore:17:*.s`.
  - `git ls-files --stage -- tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s` -> no output.
  - `git status --short --ignored=matching -- ...data_on_page_boundary_linux.s` -> `!! tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s`.
- Decision: should be force-added despite `.gitignore` because a tracked test case and tracked harness metadata require it. Leaving it ignored/untracked recreates the earlier false-negative harness failure on a clean checkout.

adversarial probes:
- stale_state: PASS. `cargo build --release` was rerun; target binary mtime is newer than the newest changed `src` file (`target_newer_or_equal=True`).
- dirty_worktree: PASS with expected caveat. The worktree is dirty by design for task 44 product changes/evidence, but tracked test/harness diff is empty. Ignored/generated items include pycache, generated object files, and the restored fixture.
- misleading_success_output: PASS. All required command exit statuses were captured; Chapter 16/15/14 summaries are green and no failing command was reworded as success.
- bridge_bypass: PASS. Forbidden bridge scan over `src/` has no matches; direct diff scan also found no `chapter_16`, `data_on_page_boundary`, `source.contains`, `gcc`, `std=c17`, `todo!`, `dbg!`, `panic!`, `unimplemented!`, `unwrap(`, or `expect(` additions in `src`.
- malformed_input: PASS. Fresh malformed probes rejected with nonzero exits: `long char` parse error, too-long string initializer type error, unterminated char lex error, and shipped `assign_to_string_literal.c` type error.
- fixture_dependency: PASS for current worktree, with durability action required. The fixture is necessary, local, ignored, and untracked; it should be `git add -f`'d before commit.

remove-ai-slops direct pass:
- Loaded and applied `omo:remove-ai-slops` criteria to the diff, tests, artifacts, and production code.
- No production test or harness diff exists, so deletion-only tests, tests that merely verify removal, excessive/useless tests, tautological tests, and implementation-mirroring tests were not introduced in the tracked test tree.
- No source-content bridge, chapter-specific source check, or fixture-name special case was found in `src/`.
- The stack-argument fix is at the typed codegen seam (`OperandType::Byte` / `OperandType::UByte`) and emits register-extension before stack push; this is not a test-name overfit.
- Existing compiler modules are oversized pre-existing architecture; this task did not introduce broad new abstractions or dependencies. The new `is_byte_type` helper is reused at multiple typed-lowering sites.

programming direct pass:
- Loaded and applied `omo:programming` plus the Rust reference criteria.
- No `unsafe`/FFI/custom lock-free code is involved in the changed Rust diff.
- Build/test/harness/manual QA gates all pass.
- No new dependency, staged file, or commit was created.
- `git diff -- src | rg` found no new `unwrap(`, `expect(`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` additions.

code-review report coverage check:
- `.omo/evidence/task-44-ch16-code-review.txt` exists and includes explicit `Programming skill / anti-slop checks` plus `Potential overfit/slop review` sections.
- The report's claims are supported by direct inspection: no test/harness diff, no bridge/system-C frontend path, no debug leftovers, no new dependencies, typed byte/string handling instead of source-text branches, and oversized-module risk documented as pre-existing/bounded.

exact evidence gaps:
- None blocking current-worktree approval.
- Durability gap before commit/clean checkout: `tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s` is required but ignored/untracked; it should be force-added with `git add -f` when committing task 44.

Final recommendation: APPROVE
