VERDICT: NEEDS-FIX

AdversarialVerify {
  task: "Task 55 W20-T5 Chapter 19 dead store elimination + default all-optimizations gate"
  verdict: needs_fix
  recommendation: REJECT
  reviewedAt: "2026-07-09"
  originalIntent: "Implement the Chapter 19 OCaml-mirrored dead store elimination pass and wire the default all-optimizations Chapter 19 gate without expanding scope into Chapter 20 or weakening the official harness."
  desiredOutcome: "A safe-to-complete Task 55 where DSE and default all-optimization official gates pass; copy-prop/UCE/fold and chapter 18 union regressions remain green; semantic DSE hazards are probed; DSE files stay below 250 pure LOC; no new compiler-phase Rust tests, dependencies, bridge/interpreter behavior, regalloc/liveness/coalescing, or unwrap/expect production escapes are introduced."
  userOutcomeReview: "Functional behavior is substantially validated: all required official gates passed, adversarial scalar/global/extern/aggregate probes passed, and the first review's extern-global deletion bug is fixed. However the shipped artifact is not safe to mark complete under the final gate because Task 55 still carries unresolved programming/remove-ai-slops debt in modified support code: src/ir/copy_propagation/rewrite.rs crosses the strict 250 pure-LOC ceiling due this task (+60 pure LOC, 244 -> 304), with non-DSE support edits also adding +121 pure LOC to an already oversized codegen.rs. The re-review reported this as WATCH; this gate treats it as a blocker because the active gate policy rejects unresolved slop/maintenance burden/scope drift, not just semantic failures."

  blockers: [
    {
      id: "B1-copy-prop-rewrite-size-regression"
      severity: "HIGH"
      evidence: "Pure LOC measurement: src/ir/copy_propagation/rewrite.rs HEAD=244, worktree=304, delta=+60. This modified support file crossed the 250 pure-LOC ceiling during Task 55. The programming and remove-ai-slops criteria classify >250 pure LOC in touched source as a defect/oversized-module smell unless split or tightly justified."
      impact: "Maintenance burden and scope drift in a non-DSE support module. This is the exact WATCH item the user asked the final gate to accept or block; I block it under the final gate policy."
      requiredFix: "Split or otherwise reduce src/ir/copy_propagation/rewrite.rs below 250 pure LOC, or remove/re-scope the non-DSE support edits so Task 55 no longer crosses the ceiling. Re-run all Task 55 gates and probes."
    },
    {
      id: "B2-non-dse-support-edit-scope"
      severity: "MEDIUM"
      evidence: "Scoped diff adds support changes outside DSE: src/codegen/codegen.rs +121 pure LOC (1907 -> 2028), src/lex/scanner.rs integer suffix logic, src/ir/copy_propagation/facts.rs, and src/ir/copy_propagation/rewrite.rs. Official gates justify some support fixes, but they are still larger than pure DSE/default wiring and increase blast radius."
      impact: "The work can pass official tests while still shipping a broader, harder-to-review patch. This reinforces B1 rather than standing alone as a semantic blocker."
      requiredFix: "After addressing B1, document or minimize each non-DSE support edit with repro-backed necessity."
    }
  ]

  checkedArtifactPaths: [
    ".omo/plans/c-compiler-rust.md",
    ".omo/evidence/task-55-dse-implementation.txt",
    ".omo/evidence/task-55-dse-code-review.md",
    ".omo/evidence/task-55-dse-fix.txt",
    ".omo/evidence/task-55-dse-code-review-2.md",
    ".omx/notepad.md",
    "src/ir/dead_store_elim/mod.rs",
    "src/ir/dead_store_elim/analysis.rs",
    "src/ir/dead_store_elim/liveness.rs",
    "src/ir/dead_store_elim/rewrite.rs",
    "src/ir/dead_store_elim/util.rs",
    "src/ir/opt.rs",
    "src/pipeline.rs",
    "src/ir/mod.rs",
    "src/ir/copy_propagation/facts.rs",
    "src/ir/copy_propagation/rewrite.rs",
    "src/codegen/codegen.rs",
    "src/lex/scanner.rs",
    "nqcc2/lib/optimizations/dead_store_elim.ml",
    "nqcc2/lib/backward_dataflow.ml",
    "nqcc2/lib/optimizations/optimize_utils.ml",
    "nqcc2/lib/optimizations/address_taken.ml",
    "nqcc2/lib/optimizations/optimize.ml"
  ]

  sourceReview: {
    planAcceptance: "Task 55 remains unchecked in the plan. Acceptance criteria are exactly the DSE-specific chapter 19 gate and default all-optimization chapter 19 gate. Guardrails require official test harness, no compiler-phase Rust unit tests, no new dependencies, no unsupported features, no Chapter 20 work, and strict OCaml mirroring."
    dseWiring: "src/ir/mod.rs registers dead_store_elim; src/ir/opt.rs adds OptPass::DeadStoreElim and invokes eliminate_dead_stores_program in fixed-point book order; src/pipeline.rs maps --eliminate-dead-stores to the pass."
    ocamlComparison: "Rust liveness/meet/rewrite structure tracks nqcc2 dead_store_elim.ml + backward_dataflow.ml: statics live at exit, calls/loads keep static+aliased vars live, calls/stores are not deleted by ordinary is_dead_store, and DSE runs after copy propagation in optimize order. Rust adds collapse_return_copies and known-memory Store/CopyBytes deletion; probes below covered the semantic watch classes."
    externGlobalFix: "analysis.rs function_static_storage_vars combines emitted static_variables with non-local function type_env names, so extern globals absent from TackyProgram.static_variables are treated as static-storage variables. liveness.rs makes static vars live at CFG exit. Manual extern scalar and aggregate probes confirmed preservation."
    dseLoc: "PASS: mod.rs=61, analysis.rs=30, liveness.rs=113, rewrite.rs=84, util.rs=148 pure LOC."
    noForbiddenRustTestsDepsEscapes: "PASS: scoped diff/new DSE scan found no .expect( or .unwrap(; no #[test]/#[cfg(test)] in src/ir src/codegen src/lex src/pipeline.rs; git diff -- tests Cargo.toml Cargo.lock produced no output; no unsafe in touched code."
    forbiddenFingerprintScans: "PASS with note: rg for bridge/interpreter/system-C found only src/ir/mod.rs:15 comment 'No runtime interpreter; the IR is consumed only by codegen and optimization.' git diff scan found no regalloc/coalescing/interference/spill/register-allocation/liveness Chapter 20 implementation fingerprints beyond DSE's own liveness module."
    codeReviewCoverage: "PRESENT: task-55-dse-code-review-2.md explicitly states omo:remove-ai-slops and omo:programming were loaded and covers overfit/slop classes (no deletion-only/tautological/implementation-mirroring tests, no new compiler-phase tests). Coverage is supported but not sufficient to override this direct gate's B1 blocker."
  }

  officialCommandEvidence: [
    { command: "git status --short", result: "dirty worktree: .omo/boulder.json modified; Task 55 source files modified; src/ir/dead_store_elim/ untracked; multiple prior evidence artifacts untracked." },
    { command: "git diff --stat", result: "8 tracked files changed, 412 insertions(+), 25 deletions(-); note: untracked DSE files are not included by git diff --stat." },
    { command: "git diff --name-status && find src/ir/dead_store_elim -maxdepth 1 -type f -print", result: "Tracked modified files: .omo/boulder.json, src/codegen/codegen.rs, src/ir/copy_propagation/{facts.rs,rewrite.rs}, src/ir/mod.rs, src/ir/opt.rs, src/lex/scanner.rs, src/pipeline.rs. Untracked DSE files: analysis.rs, liveness.rs, mod.rs, rewrite.rs, util.rs." },
    { command: "cargo fmt --all -- --check", exit: 0 },
    { command: "cargo check --release", exit: 0, output: "Finished release profile target(s) in 0.03s" },
    { command: "cargo build --release", exit: 0, output: "Finished release profile target(s) in 0.01s" },
    { command: "cargo test --release", exit: 0, output: "10 main tests passed; lib/doc tests 0 passed/0 failed" },
    { command: "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores", exit: 0, output: "Ran 27 tests in 0.592s OK" },
    { command: "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only", exit: 0, output: "Ran 120 tests in 2.826s OK" },
    { command: "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies", exit: 0, output: "Ran 42 tests in 0.965s OK" },
    { command: "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code", exit: 0, output: "Ran 15 tests in 0.333s OK" },
    { command: "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants", exit: 0, output: "Ran 16 tests in 0.420s OK" },
    { command: "./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union", exit: 0, output: "Ran 286 tests in 5.060s OK; assembler truncation warnings for two chapter_18 static initializer client .s files preserved as non-blocking harness warnings." },
    { command: "git diff --check", exit: 0 }
  ]

  locEvidence: [
    "src/ir/dead_store_elim/mod.rs 61",
    "src/ir/dead_store_elim/analysis.rs 30",
    "src/ir/dead_store_elim/liveness.rs 113",
    "src/ir/dead_store_elim/rewrite.rs 84",
    "src/ir/dead_store_elim/util.rs 148",
    "src/ir/copy_propagation/rewrite.rs HEAD=244 WT=304 delta=+60 BLOCKER",
    "src/codegen/codegen.rs HEAD=1907 WT=2028 delta=+121 WATCH/B2",
    "src/lex/scanner.rs HEAD=557 WT=557 delta=0",
    "src/ir/copy_propagation/facts.rs HEAD=177 WT=178 delta=+1",
    "src/ir/opt.rs HEAD=39 WT=44 delta=+5",
    "src/pipeline.rs HEAD=92 WT=95 delta=+3",
    "src/ir/mod.rs HEAD=12 WT=13 delta=+1"
  ]

  manualProbeEvidence: [
    { class: "dead overwritten locals are removed", source: "/tmp/task55_dead_local.c", result: "baseline=2, dse=2, all=2; DSE TACKY main body is Return(Constant(2)) with no x stores." },
    { class: "address-taken locals preserved", source: "/tmp/task55_address_taken.c", result: "baseline=2, dse=2, all=2" },
    { class: "live stores through pointers preserved", source: "/tmp/task55_pointer_store_live.c", result: "baseline=7, dse=7, all=7" },
    { class: "calls with side effects preserved despite dead return value", source: "/tmp/task55_call_side_effect.c", result: "baseline=9, dse=9, all=9" },
    { class: "file-scope global scalar store preserved", source: "/tmp/task55_global_same_tu.c", result: "baseline=6, dse=6, all=6" },
    { class: "file-scope aggregate global CopyBytes preserved", source: "/tmp/task55_aggregate_global.c", result: "baseline=11, dse=11, all=11" },
    { class: "first review extern-global scalar bug fixed", sources: "/tmp/task55_extern_store.c + /tmp/task55_extern_client.c", result: "baseline=5, dse=5, all=5; all-opts TACKY set_g preserved Copy { src: Constant(5), dst: \"g\" }." },
    { class: "extern aggregate global CopyBytes preserved", sources: "/tmp/task55_extern_agg_store.c + /tmp/task55_extern_agg_client.c", result: "baseline=7, dse=7, all=7; all-opts TACKY set_g preserved GetAddress g and CopyBytes size 8." },
    { class: "malformed input smoke", source: "/tmp/task55_malformed.c", result: "rustcc --eliminate-dead-stores exited 1 with 'type error: function with non-void return type must return a value'." }
  ]

  ultraqaNotes: {
    dirty_worktree: "Present before this review: .omo/boulder.json modified, Task 55 source changes uncommitted, src/ir/dead_store_elim untracked, numerous prior evidence artifacts untracked. This review did not edit product files, docs, plans, Boulder state, or git history."
    stale_state: "Plan Task 55 checkbox remains unchecked. .omx/notepad.md exists but contains old Ralph history and no Task 55-specific evidence; not used as completion proof."
    misleading_success_output: "Official gates and prior PASS review are real but insufficient alone. git diff --stat omits the untracked DSE implementation, and re-review's WATCH item is a final-gate blocker under active anti-slop policy."
    malformed_input: "Malformed return probe failed cleanly with exit 1; no hang."
    hung_commands: "No command hung; all official gates and probes completed within the observed shell timeouts."
    flaky_tests: "No flake observed in this single adversarial run; official harness results matched prior evidence."
    generated_artifacts_temp_cleanup: "Manual probes created /tmp/task55_* C files, .s files, and binaries. The first cleanup left older /tmp/task55_* artifacts from prior reviewers visible; final cleanup command rm -f /tmp/task55_* /tmp/run_task55_probes.sh removed all, verified by empty ls /tmp/task55_* output."
  }

  evidenceGaps: [
    "No product code fixes were made by this read-only gate. B1 remains unresolved.",
    "No Boulder state inspection was used for approval because the user forbade Boulder edits and source/test artifacts were sufficient for the gate decision.",
    "Manual probes are temporary adversarial checks, not official tests, per project policy."
  ]
}
