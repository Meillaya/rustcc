AdversarialVerify report: task 44 / W17-T1 Chapter 16 characters and string literals
Date: 2026-07-08
Workspace: /home/mei/projects/rustcc
Recommendation: REJECT
Status: BLOCKED by harness fixture; not a demonstrated product compiler failure.

originalIntent:
- Implement Chapter 16 native compiler support for characters and string literals.
- Acceptance from .omo/plans/c-compiler-rust.md task 44 requires:
  1. `int main(void) { char c = 'A'; return c; }` exits 65.
  2. `int main(void) { char *s = "hello"; return s[0]; }` exits 104.
  3. `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` is green.

desiredOutcome:
- The user can advance task 44 only after the Chapter 16 latest-only gate is green without weakening tests or relying on bridge/system-C bypasses.

userOutcomeReview:
- Manual acceptance examples pass with the native compiler.
- Chapter 14 and 15 regression latest-only gates pass.
- cargo build/test/fmt pass.
- However, the required Chapter 16 latest-only acceptance gate exits 1, so the shipped artifact does not fully satisfy the user's stated outcome. Because the only observed Chapter 16 failure is a missing harness assembly fixture, this is classified as harness fixture blockage rather than a product compiler failure. It is still not APPROVE because the plan explicitly requires the green Chapter 16 command before advancing.

checked artifact paths:
- .omo/plans/c-compiler-rust.md (task 44 lines 1638-1662)
- .omo/evidence/task-44-ch16-implementation.txt
- .omo/evidence/task-44-ch16-code-review.txt
- tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c
- tests/test_framework/basic.py
- tests/test_properties.json
- git diff: src/ast/expr.rs, src/ast/ty.rs, src/codegen/assembly.rs, src/codegen/codegen.rs, src/codegen/emit.rs, src/codegen/replace_pseudos.rs, src/compiler.rs, src/ir/lower.rs, src/ir/tacky.rs, src/parse/parser.rs, src/semantics/resolve.rs, src/semantics/typecheck.rs

changed-files / diff review:
- Product files modified: 12 Rust files under src/.
- Tests/harness modified in git diff: none. `git diff --name-status -- tests tests/test_framework` produced no paths.
- No untracked generated outputs were left under tests/ or tests/test_framework after the harness runs.
- Suspicious diff scan found no bridge/system frontend strings in src. It did flag two direct-review notes, not blockers for this verdict: StringLiteral lvalue treatment and a defensive `%xmm?` formatter fallback for impossible byte-XMM registers.
- remove-ai-slops/programming direct pass: no test deletion, no harness weakening, no source-content special cases for chapter_16/push_arg/data_on_page_boundary, no dbg!/todo!/panic!/unwrap added in diff. Existing compiler phase files were already oversized before this task and grew further; this is a maintenance risk but the blocking failure below is the required Chapter 16 gate.
- Code review artifact coverage: .omo/evidence/task-44-ch16-code-review.txt explicitly includes "Programming skill / anti-slop checks" and "Potential overfit/slop review" sections covering bridge bypass, harness weakening, debug leftovers, oversized-module risk, byte ops, static strings, char initializers, and typed lowering/codegen.

required command results:
1. `cargo build --release`
   - Exit: 0
   - Output: `Finished release profile [optimized] target(s) in 0.01s`

2. `cargo test --release`
   - Exit: 0
   - Output: 10 main tests passed; doc-tests passed.

3. `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only`
   - Exit: 1
   - Result: FAIL / BLOCKED
   - Exact failure:
     `RuntimeError: /home/mei/projects/rustcc/tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s: Assembler messages:`
     `Error: can't open /home/mei/projects/rustcc/tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s for reading: No such file or directory`
   - Harness summary: `Ran 72 tests in 1.625s` / `FAILED (errors=1)`.
   - Product vs harness: harness fixture missing. No product compiler failures were reported by this run; all other Chapter 16 tests completed.

4. `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only`
   - Exit: 0
   - Output: `Ran 83 tests ... OK`.

5. `./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only`
   - Exit: 0
   - Output: `Ran 53 tests ... OK`.

6. forbidden bridge scan:
   - Command: `rg -n "gcc_array_subset_assembly|source\\.contains\\(\\\"\\[|-std=c17|system_c_to_assembly|compile_with_system_cc_frontend|sanitize_system_assembly|evaluate_with_system_cc|system_c_syntax_check|should_defer_parse_to_system_frontend|source_has_|likely_parse_error|semantic_error_that_should_parse" src; printf 'forbidden_bridge_scan_exit=%s\n' "$?"`
   - Exit: 0 wrapper / ripgrep no-match exit 1
   - Output: `forbidden_bridge_scan_exit=1`
   - Classification: clean for src/ product bridge bypass.

additional verification:
- `cargo fmt -- --check` exited 0.
- Target freshness probe: target/release/rustcc mtime 2026-07-08 15:55:33 -0400; newest changed src mtime 2026-07-08 15:55:20 -0400; target_newer_or_equal=true.

manual acceptance probes:
1. Source: `int main(void) { char c = 'A'; return c; }`
   - Compile: `./target/release/rustcc /tmp/.../char_exit.c` exit 0.
   - Run: exit 65.

2. Source: `int main(void) { char *s = "hello"; return s[0]; }`
   - Compile: `./target/release/rustcc /tmp/.../string_index.c` exit 0.
   - Run: exit 104.

3. Native stage probe:
   - `./target/release/rustcc --tacky /tmp/.../string_index.c` output includes `GetAddress { src: "string.0" }`, `Load` with `tmp.2: Byte`, `SignExtend` to `Int`, and `static_constants` bytes `[104, 101, 108, 108, 111, 0]`.
   - `./target/release/rustcc --codegen /tmp/.../char_exit.c` output includes byte codegen: `movb $65`, `movb -4(%rbp), %r10b`, `movsbl -8(%rbp), %r10d`.

fixture investigation:
- Failing C test: `tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c` declares `extern char zed; // defined in data_on_page_boundary.s` and passes `zed` as the 7th char argument to ensure byte-width stack argument handling near a page boundary.
- Harness behavior: `tests/test_framework/basic.py` `get_libs()` reads `assembly_libs` from `tests/test_properties.json`, appends platform suffix `_linux.s`, and `library_test_helper()` compiles the C file under test with rustcc `-c`, then invokes GCC with the rustcc object plus the assembly dependency.
- `tests/test_properties.json` maps `chapter_16/valid/chars/push_arg_on_page_boundary.c` to `chapter_16/valid/chars/data_on_page_boundary`.
- On Linux the harness therefore expects `tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s`.
- `git ls-files` contains `tests/tests/chapter_16/valid/chars/push_arg_on_page_boundary.c` but not `tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s`.
- `find . -path './target' -prune -o -name '*data_on_page_boundary*' -print` found no fixture files anywhere in the checkout.
- Native compile-only of a temp copy of `push_arg_on_page_boundary.c` with `./target/release/rustcc -c` exited 0 and produced an object. The failing step is GCC opening the missing assembly fixture, not rustcc compiling the C file.
- Classification: missing checked-out/generated harness fixture. It is not a product compiler failure, but it blocks the required Chapter 16 latest-only command.

adversarial classes:
- stale_state: PASS. `cargo build --release` was run; target binary is newer than changed source mtimes.
- dirty_worktree: PASS with caveat. Worktree is dirty by design with 12 product Rust files and untracked evidence artifacts; no tests/harness diff and no test output artifacts under tests/.
- misleading_success_output: PASS. Chapter 16 command clearly returns exit 1 and reports `FAILED (errors=1)`; do not treat executor's blocked wording as green.
- bridge_bypass: PASS. Forbidden bridge scan over src found no product bypass strings.
- malformed_input: PASS. Temp malformed probes were rejected: `long char` parse error, too-large string initializer type error, and unterminated string lex error.
- harness_fixture_missing: FAIL/BLOCKED. Required Chapter 16 latest-only gate cannot be green until missing `data_on_page_boundary_linux.s` fixture issue is resolved or otherwise accounted for without weakening tests.

blockers:
- Blocking command: `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only`
- Blocking exit: 1
- Root cause: missing assembly fixture `tests/tests/chapter_16/valid/chars/data_on_page_boundary_linux.s` referenced by `tests/test_properties.json` and required by `tests/test_framework/basic.py` on Linux.
- Classification: harness fixture missing/generated artifact expectation, not product compiler failure observed.
- User-facing impact: Task 44 acceptance criterion 3 is unmet; do not advance/approve task 44 until the Chapter 16 latest-only command exits 0 without test/harness weakening.

exact evidence gaps:
- There is no green Chapter 16 latest-only run in this checkout.
- There is no checked-in `data_on_page_boundary_linux.s` fixture in the repo, and I did not create it because this verification is read-only except evidence.
- Because the Chapter 16 harness is blocked before a full green run, completion remains unproven even though observed product behavior and regressions pass.

Final recommendation: REJECT
