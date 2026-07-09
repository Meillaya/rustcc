AdversarialVerify {
  task: "56. W21-T1: Chapter 20 - liveness analysis via backward dataflow"
  cwd: "/home/mei/projects/rustcc"
  reviewer_role: "independent adversarial gate; read-only except evidence"
  recommendation: "REJECT"
  verdict: "NEEDS-FIX"

  originalIntent: "Implement the Chapter 20 liveness foundation over the assembly CFG, mirroring nqcc2/lib/backend/regalloc.ml liveness sufficiently for later register allocation. Do not implement W21-T2+ interference/color/spill/coalesce."
  desiredOutcome: "Task 56 can be checked only if the small liveness example matches the OCaml/hand reference, required regressions pass, source remains scoped and hygienic, dataflow handles branches/calls/memory/division/GP-XMM classes conservatively, and evidence is complete."
  userOutcomeReview: "The branch/dataflow probe and official regression gates pass, but the shipped RegisterClass hard-register sets do not mirror the OCaml reference and include scratch/reserved registers used elsewhere by the backend. This is not conservative enough for later regalloc and makes the success prose misleading. The task-56 code review report now present in evidence also rejects on the same register-class issue and call metadata concerns. The user should receive NEEDS-FIX, not completion."

  checkedArtifactPaths: [
    ".omo/plans/c-compiler-rust.md:1894-1910",
    ".omo/evidence/task-56-liveness-implementation.txt",
    ".omo/evidence/task-56-liveness-code-review.md",
    "src/codegen/regalloc/types.rs",
    "src/codegen/regalloc/operands.rs",
    "src/codegen/regalloc/liveness.rs",
    "src/codegen/regalloc/mod.rs",
    "src/codegen/assembly.rs",
    "src/codegen/fixup.rs",
    "src/codegen/replace_pseudos.rs",
    "src/codegen/codegen.rs",
    "nqcc2/lib/backend/regalloc.ml:89-145,227-263,610-634",
    "/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md",
    "/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md",
    "/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md",
    "/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md"
  ]

  blockers: [
    {
      id: "register-class-ocaml-reference-mismatch"
      severity: "blocking"
      evidence: "src/codegen/regalloc/types.rs includes GP R10/R11 in all_hardregs and caller_saved_regs, and XMM14/XMM15 in all_hardregs/caller_saved_regs. OCaml reference nqcc2/lib/backend/regalloc.ml has GP all_hardregs=[AX;BX;CX;DX;DI;SI;R8;R9;R12;R13;R14;R15], GP caller_saved=[AX;CX;DX;DI;SI;R8;R9], XMM all_hardregs=XMM0..XMM13, and XMM caller_saved=all_hardregs."
      why_it_blocks: "R10/R11/XMM14/XMM15 are scratch/reserved in this Rust backend (grep shows R10/R11/XMM14/XMM15 use in codegen/fixup/replace_pseudos). Including them in allocatable/caller-saved class sets fails the 'matches OCaml reference' and 'GP/XMM class behavior conservatively enough for later regalloc' criteria."
      probe_result: "reference regclass probe exited 42 with REFERENCE_MISMATCH for all four class sets."
    },
    {
      id: "call-metadata-defaults-to-no-params"
      severity: "blocking-risk"
      evidence: "src/codegen/regalloc/operands.rs uses config.call_param_regs.get(name).map_or_else(Vec::new, ...) for Instr::Call. The public analyze_function_liveness accepts fn_name but does not derive call/return metadata from project symbols. The inspected task-56 code-review report flags this as HIGH-2."
      why_it_blocks: "A missing callee entry is silently analyzed as using no parameter registers. That can make call liveness wrong while official chapter gates still pass because the liveness pass is not wired into normal compilation yet. The expected outcome explicitly requires calls to be handled conservatively."
    }
  ]

  codeReviewReportCoverage: {
    path: ".omo/evidence/task-56-liveness-code-review.md"
    verdict: "REJECT"
    skill_perspective_check: "Present. Report explicitly states it consulted omo:remove-ai-slops and omo:programming, including overfit/slop and Rust criteria."
    support_for_gate: "Supports NEEDS-FIX: HIGH-1 register-class mismatch, HIGH-2 missing callee metadata, medium/low semantic drift notes."
  }

  passes: [
    "Plan acceptance small branch liveness output matched hand-derived sets for live_in/live_out.",
    "Official regressions passed: cargo fmt/check/build/test, chapter 19 default, chapter 19 DSE, chapter 18 union, git diff --check.",
    "No new Cargo.toml/Cargo.lock dependency changes.",
    "No unsafe/unwrap/expect found in task source scan.",
    "No new test/spec files found in task diff/untracked scan.",
    "No system-C bridge/gcc/clang/std::process additions found in task files; grep hit only the word 'system' in an assembly.rs comment and 'succs' field names.",
    "No W21-T2+ implementation observed: no interference graph, coloring, spill rewrite loop, or coalescing implementation; allocate() remains the chapter-20 stub.",
    "Pure LOC under 250 for each new/modified Rust file: assembly.rs 204, mod.rs 21, types.rs 66, operands.rs 130, liveness.rs 102."
  ]

  directRemoveAiSlopAndProgrammingPass: {
    scope: "Task 56 branch source and probes; read-only."
    overfit_slop_criteria: [
      "No tests were added, so no excessive/useless/deletion-only/tautological/implementation-mirroring new tests found.",
      "No new dependencies, unsafe, unwrap, expect, or bridge/system-C additions found.",
      "No unnecessary production extraction/parsing/normalization found in the liveness modules; split into types/operands/liveness is scoped and each file is under 250 pure LOC.",
      "Maintenance-risk blocker remains: RegisterClass hardreg sets drift from OCaml and include backend scratch registers."
    ]
    programming_criteria: [
      "Rust files are below 250 pure LOC.",
      "No escape hatches (unsafe/unwrap/expect) in changed task source.",
      "No code edits were made by this verifier."
    ]
  }

  ultraqaNotes: {
    dirty_worktree: "Applicable. Pre-write git status showed modified src/codegen/assembly.rs and src/codegen/regalloc/mod.rs; untracked src/codegen/regalloc/{liveness,operands,types}.rs; older unrelated untracked .omo evidence/start-work files. I did not stage/commit/mark complete. This evidence artifact is an allowed new evidence file."
    stale_state: "Applicable. Plan .omo/plans/c-compiler-rust.md still has Task 56 unchecked. Implementation evidence and code-review evidence were read fresh."
    misleading_success_output: "Applicable. Implementation evidence claimed all gates/probe pass, but independent reference-regclass probe found OCaml mismatch after green official gates; code-review report also rejects."
    malformed_noisy_command_applicability: "Applicable only to disposable probe harness. First /tmp rustc probe failed because include! placed an inner #![allow(dead_code)] attribute after items: 'error: an inner attribute is not permitted in this context' [exit 1]. I fixed only the /tmp harness and reran. Later rustc dead_code warnings were from disposable probe stubs and did not affect product code."
    hung_commands: "No hung commands. Official gates were timeout-wrapped at 180s and manual probes at 60s; no timeout exit 124 occurred."
    flaky_tests: "No flake observed. Official gates ran once fresh and passed; no inconsistent retry results."
    temp_cleanup: "Created disposable /tmp/task56_* scripts/binaries/logs for probes; cleanup command removed them. find /tmp -maxdepth 1 -name 'task56*' printed no remaining files."
  }

  commandResults: {
    git_status_initial: "git status --short --untracked-files=all => M src/codegen/assembly.rs; M src/codegen/regalloc/mod.rs; ?? src/codegen/regalloc/liveness.rs; ?? src/codegen/regalloc/operands.rs; ?? src/codegen/regalloc/types.rs; ?? .omo/evidence/task-56-liveness-implementation.txt; plus older unrelated untracked .omo evidence/start-work files."

    official_gates: [
      "cargo fmt --all -- --check: [exit 0]",
      "cargo check --release: Finished `release` profile [optimized] target(s) in 0.03s; [exit 0]",
      "cargo build --release: Finished `release` profile [optimized] target(s) in 0.01s; [exit 0]",
      "cargo test --release: src/lib.rs 0 tests ok; src/main.rs 10 tests ok; doc-tests 0 tests ok; [exit 0]",
      "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only: Ran 120 tests in 2.805s; OK; [exit 0]",
      "./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores: Ran 27 tests in 0.647s; OK; [exit 0]",
      "./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union: Ran 286 tests in 5.162s; OK; assembler warnings for two chapter_18 static initializer clients about value truncation; [exit 0]",
      "git diff --check: no output; [exit 0]"
    ]

    source_hygiene_scans: [
      "pure LOC: src/codegen/assembly.rs: 204; src/codegen/regalloc/mod.rs: 21; src/codegen/regalloc/types.rs: 66; src/codegen/regalloc/operands.rs: 130; src/codegen/regalloc/liveness.rs: 102",
      "grep unsafe/unwrap/expect in task files: no output",
      "manifest diff names for Cargo.toml/Cargo.lock: no output",
      "new test file scan in diff/untracked: no output",
      "scope keyword scan found only comments/stub text mentioning interference/coloring/spilling/coalescing; no W21-T2+ implementation."
    ]

    manual_liveness_probe: {
      compile: "rustc --edition=2024 /tmp/task56_liveness_probe.rs -o /tmp/task56_liveness_probe: warning-only from disposable stub modules; [exit 0]"
      run: "branch block0 live_in: {Pseudo(\"a\"), Pseudo(\"c\")}; branch block0 live_out: {Pseudo(\"b\"), Pseudo(\"c\")}; branch block1 live_in: {Pseudo(\"b\"), Pseudo(\"c\")}; branch block1 live_out: {Pseudo(\"b\")}; branch block2 live_in: {Pseudo(\"b\")}; branch block2 live_out: {Reg(AX)}; memory dst reads pointer regs and src: {Reg(R12), Reg(R13), Pseudo(\"v\")}; memory dst writes no liveness-kill operand: {}; idiv uses divisor address plus AX/DX: {Reg(AX), Reg(DX), Reg(R12), Reg(R13)}; idiv writes AX/DX: {Reg(AX), Reg(DX)}; div uses divisor reg plus AX/DX: {Reg(AX), Reg(DX), Reg(R8)}; div writes AX/DX: {Reg(AX), Reg(DX)}; idivq uses divisor reg plus AX/DX: {Reg(AX), Reg(DX), Reg(R9)}; idivq writes AX/DX: {Reg(AX), Reg(DX)}; divq uses divisor reg plus AX/DX: {Reg(AX), Reg(DX), Reg(R12)}; divq writes AX/DX: {Reg(AX), Reg(DX)}; call GP params filtered to GP: {Reg(DI), Reg(SI), Reg(R9)}; call GP caller-saved writes actual: {Reg(AX), Reg(CX), Reg(DX), Reg(DI), Reg(SI), Reg(R8), Reg(R9), Reg(R10), Reg(R11)}; call XMM params filtered to XMM: {Reg(XMM(0))}; call XMM caller-saved writes actual: {Reg(XMM(0)), Reg(XMM(1)), Reg(XMM(2)), Reg(XMM(3)), Reg(XMM(4)), Reg(XMM(5)), Reg(XMM(6)), Reg(XMM(7)), Reg(XMM(8)), Reg(XMM(9)), Reg(XMM(10)), Reg(XMM(11)), Reg(XMM(12)), Reg(XMM(13)), Reg(XMM(14)), Reg(XMM(15))}; PASS: branch, memory, idiv/div implicit-reg shape, call param filtering, and class filtering probe matched expected current semantics; [exit 0]"
    }

    reference_regclass_probe: {
      compile: "rustc --edition=2024 /tmp/task56_reference_regclass_probe.rs -o /tmp/task56_reference_regclass_probe: warning-only from disposable stub modules; [exit 0]"
      run: "GP all_hardregs vs nqcc2/lib/backend/regalloc.ml:610: REFERENCE_MISMATCH actual [AX, BX, CX, DX, DI, SI, R8, R9, R10, R11, R12, R13, R14, R15] expected [AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15]; GP caller_saved_regs vs line 611: REFERENCE_MISMATCH actual [AX, CX, DX, DI, SI, R8, R9, R10, R11] expected [AX, CX, DX, DI, SI, R8, R9]; XMM all_hardregs vs lines 616-632: REFERENCE_MISMATCH actual [XMM(0)..XMM(15)] expected [XMM(0)..XMM(13)]; XMM caller_saved_regs vs line 634: REFERENCE_MISMATCH actual [XMM(0)..XMM(15)] expected [XMM(0)..XMM(13)]; FAIL: register-class hardreg sets do not mirror OCaml reference; scratch/reserved regs are included by Rust implementation; [exit 42]"
    }

    temp_cleanup: "rm -f /tmp/task56_*; find /tmp -maxdepth 1 -name 'task56*' -print => no output"
  }

  evidenceGaps: [
    "No separate task-56 manual QA matrix artifact was supplied beyond implementation/code-review evidence and this adversarial probe.",
    "No task-56-specific notepad path was supplied; .omx/notepad.md exists but was not task-specific evidence."
  ]
}

VERDICT: NEEDS-FIX
