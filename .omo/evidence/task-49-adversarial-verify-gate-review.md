# Task 49 gate review

recommendation: REJECT

## blockers

1. Missing task49 code-review/slop artifact. The only task49-specific artifact found before this review was `.omo/evidence/task-49-ch18-union-implementation.txt`; no `.omo/evidence/task-49-*code-review*` or `.omo/evidence/task-49-*slop*` report exists, and `.omo/evidence/review-union-core/*` contains command output snapshots rather than a code-review/slop report with `remove-ai-slops` and `programming` coverage.

## originalIntent

Add W19-T2 Chapter 18 union core: native `Type::Union`, parser/resolver tag-kind handling, union layout with max size/alignment and member offset 0, typecheck/lowering for union initialization/copy/member access.

## desiredOutcome

Approve only if union core is complete and remaining full `--union` failures are W19-T3 ABI-only; otherwise reject with command/root-cause evidence.

## userOutcomeReview

Functional union-core verification passed: build, unit tests, ch18 `--union --stage validate`, ch18 `--union --stage codegen`, manual size/alias/member-error probes, and tag-scope/malformed adversarial probes are green. Full ch18 `--union` still fails, but every failure/error path is in W19-T3 parameter/return/library ABI buckets. Approval is still blocked because the required task49 code-review/slop artifact is missing.

## checked artifact paths

- `.omo/plans/c-compiler-rust.md` (tasks 49-50)
- `.omo/evidence/task-49-ch18-union-implementation.txt`
- `.omo/evidence/review-union-core/*`
- `.omx/notepad.md`
- current git diff for `src/ast/decl.rs`, `src/ast/mod.rs`, `src/ast/ty.rs`, `src/codegen/type_table.rs`, `src/ir/lower.rs`, `src/parse/parser.rs`, `src/semantics/resolve.rs`, `src/semantics/typecheck.rs`
- `tests/` tracked diff/status

## command evidence

- `cargo build --release` -> PASS
- `cargo test --release` -> PASS, 10 binary tests
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union --stage validate` -> PASS, 286 tests
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union --stage codegen` -> PASS, 286 tests
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` -> FAIL, 286 tests, 31 failures, 5 errors; classifier found 36/36 failures in W19-T3 ABI buckets and 0 unexpected union-core failures
- `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only` -> PASS, 70 tests
- `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` -> PASS, 72 tests
- forbidden bridge grep over `src` -> PASS/no output
- `git diff --name-status -- tests` and `git status --short -- tests` -> no output

## exact evidence gaps

- Missing task49 code-review/slop artifact with explicit `remove-ai-slops` overfit/slop criterion coverage and `programming` Rust criteria coverage.

## recommendation

REJECT
