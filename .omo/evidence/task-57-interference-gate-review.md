# Task 57 W21-T2 Interference Graph Gate Review

recommendation: REJECT

## originalIntent
Implement W21-T2: Chapter 20 interference graph construction and simplification, matching OCaml regalloc behavior for graph edges, low-degree pruning, spill-candidate fallback, hard-register/register-class constraints, and static/address-taken conservatism.

## desiredOutcome
A user should be able to mark Task 57 complete because source, tests, manual QA, code review, and official gates collectively prove the graph/simplification behavior and show no scope drift or maintenance slop.

## userOutcomeReview
The shipped source appears to implement the core graph/simplify mechanics, and an independent /tmp probe confirmed manual edges, move suppression, GP/XMM class behavior, hard-register pressure, and config-based static/address-taken exclusion. However the user cannot safely mark the task complete: no durable checked-in regression tests cover the new behavior, the implementation probe named in evidence is not present in the repo, the required code-review/slop-coverage artifact is absent, clippy is not green, and parameter-bloat slop remains undocumented.

## checked artifact paths
- `.omo/evidence/task-57-interference-implementation.txt`
- `.omo/evidence/task-57-interference-adversarial-verify.txt`
- `.omo/plans/c-compiler-rust.md`
- `.omx/notepad.md`
- `.omo/start-work/ledger.jsonl`
- `docs/stages/ch20-register-allocation.md`
- `docs/book/ch20-register-allocation.md`
- `docs/specs/optimization-and-regalloc-requirements.md`
- `nqcc2/lib/backend/regalloc.ml`
- `nqcc2/lib/optimizations/address_taken.ml`
- `src/codegen/regalloc/mod.rs`
- `src/codegen/regalloc/operands.rs`
- `src/codegen/regalloc/graph.rs`
- `src/codegen/regalloc/simplify.rs`
- Supporting inspected files: `src/codegen/regalloc/liveness.rs`, `src/codegen/regalloc/types.rs`, `src/codegen/assembly.rs`, `src/ir/tacky.rs`, `src/ir/cfg.rs`.

## blockers
1. No Task 57 code review report exists, so required remove-ai-slops/programming and overfit/slop coverage is absent.
2. No checked-in regression tests cover `build_interference`, `InterferenceGraph`, `SimplifyChoice`, or `simplify`.
3. `.omo/evidence/task-57-interference-implementation.txt` cites `cargo run --release --bin regalloc_probe`, but current repo has no `src/bin/regalloc_probe.rs`; the manual probe artifact is non-reproducible.
4. `cargo clippy --all-targets --all-features -- -D warnings` fails with 31 project-level errors.
5. New graph API/helper signatures include 6/5/4 parameter functions without a typed context object or documented exception, violating the programming parameter-bloat criterion.
6. Static/address-taken behavior is only proven through manual `InterferenceConfig`; no integration path populates it in allocation yet.

## exact evidence gaps
- Missing files: `.omo/evidence/*task-57*review*` and `.omo/evidence/*57*gate-review*` (before this review) produced no matches.
- Grep evidence for checked-in tests found only source exports and the implementation evidence reference; no source/test regression for graph/simplify.
- Fresh command evidence is recorded in `.omo/evidence/task-57-interference-adversarial-verify.txt`; key failures are clippy exit 101 and chapter 20 no-coalescing exit 1.

## positive evidence
- `cargo fmt --all -- --check`, `cargo check --release`, `cargo build --release`, `cargo test --release`, chapter 19 latest, chapter 19 DSE, chapter 18 union, and `git diff --check` all exited 0.
- Independent /tmp probe exited 0 and validated expected manual edges `{a-b, a-c, b-c, d-e}`, low-degree simplification, forced spill-candidate behavior, GP/XMM class filtering, move-source suppression, hardreg DX interference, and static/aliased exclusion.
- Changed regalloc files are under the 250 pure LOC ceiling and contain no unsafe/unwrap/debug leftovers.

## recommendation rationale
REJECT until the missing review artifact and durable tests are added, the non-reproducible probe evidence is replaced by checked-in tests or a reproducible test command, and the quality/slop issues are either fixed or explicitly documented as pre-existing/out-of-scope with a green task-specific gate.
