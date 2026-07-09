VERDICT: NEEDS-FIX
recommendation: REJECT

Task: 58 / W21-T3 coloring + select phase
Gate date: 2026-07-09
Mode: adversarial read-only gate; only evidence files written by this gate.

originalIntent
- Implement Chapter 20 W21-T3 select/coloring after W21-T2 simplification.
- Match the OCaml `nqcc2/lib/backend/regalloc.ml` select/color behavior closely enough for downstream no-coalescing register allocation.
- Preserve W21-T4+ scope boundaries: no spill rewrite/reallocation loop and no conservative coalescing in this slice.

desiredOutcome
- User can mark Task 58 complete only if the durable coloring probe, official gates, scope fidelity, OCaml parity, reserved register behavior, spill marker behavior, and LOC/hygiene all support completion.

userOutcomeReview
- The user should NOT mark Task 58 complete. The source adds a select/color API and the durable probe runs, but the official Task 58 plan gate is red, `allocate` is still not wired, exact OCaml select/color parity is not established and appears violated, and the probe is too overfit/manual to provide durable confidence.
- Positive evidence: `cargo fmt`, `cargo check`, `cargo build`, `cargo test`, chapter 18/19 regression gates, rustfmt on the durable probe, rustc+run of the durable probe, and scoped whitespace checks passed. Source inspection confirms reserved GP/XMM registers are excluded in `RegisterClass`; spill marker behavior returns `None` when no color is available.
- Blocking evidence: chapter 20 `--no-coalescing` official gate failed; OCaml color mapping gives caller-saved hardregs low colors while Rust precolors directly by `all_hardregs` order; Task 58 has no real Cargo-integrated regression/unit test and no pre-existing Task 58 code-review artifact.

checked artifact paths
- `.omo/evidence/task-58-coloring-implementation.txt`
- `.omo/evidence/task-58-coloring-probe.rs`
- `.omo/evidence/task-58-coloring-adversarial-verify.txt` (this artifact)
- `.omo/evidence/task-58-coloring-gate-review.md` (same review mirrored for final-gate contract)
- `.omo/plans/c-compiler-rust.md` lines 1926-1938
- `.omx/notepad.md` (stale historical ch20 notes observed; not trusted for this gate)
- `.omo/start-work/ledger.jsonl` (no Task 58 completion entry found)
- `src/codegen/regalloc/color.rs`
- `src/codegen/regalloc/mod.rs`
- `src/codegen/regalloc/types.rs`
- `src/codegen/regalloc/graph.rs`
- `src/codegen/regalloc/simplify.rs`
- `src/codegen/assembly.rs`
- `nqcc2/lib/backend/regalloc.ml` lines 470-563 and 607-637
- `nqcc2/lib/assembly.ml` lines 3-46

blockers
1. Official acceptance gate is red. Plan Task 58 acceptance says `--chapter 20 --latest-only --no-coalescing` must pass; fresh run exited 1 with `Ran 66 tests ... FAILED (failures=2, errors=46)`.
2. `src/codegen/regalloc/mod.rs:72-75` still has `allocate(_asm) -> unimplemented!("ch.20 regalloc wired in wave 21")`, so the user-visible Chapter 20 register allocation path is not complete.
3. OCaml select/color parity is not satisfied. Rust `color.rs:45-70` maps color index directly to `RegisterClass::all_hardregs()` order, so low colors allocate `AX`, then `BX`; OCaml `regalloc.ml:532-538` assigns callee-saved hardregs the highest available color and caller-saved hardregs the lowest before `make_register_map` (`546-563`). A direct simulation from those OCaml lines and `assembly.ml` register order gives color 0 -> `R9`, color 1 -> `R8`, ... color 6 -> `AX`, color 7 -> `BX`; Rust durable probe reports small graph `{Pseudo("a"): Some(BX), Pseudo("b"): Some(AX)}`.
4. Durable probe is overfit/manual-only. `.omo/evidence/task-58-coloring-probe.rs` uses copied stubs plus an absolute `#[path]` to `src/codegen/regalloc/color.rs`; it does not compile production `types.rs`/`graph.rs`, does not assert `used_callee_saved_regs`, and its reserved-reg output comes from stubbed `RegisterClass` rather than the production one.
5. Missing durable tests/code-review artifact. No Cargo-integrated unit/regression test covers `select`, and there was no pre-existing `.omo/evidence/task-58-*code-review*` or Task 58 gate-review artifact before this adversarial gate. The spawned read-only code reviewer also returned REJECT.
6. Hygiene/slop watch: `color_node` has 5 parameters and `remove_neighbor_color` has 4, triggering the programming parameter-bloat smell. This is secondary to the functional blockers but should be fixed with a small typed context or otherwise justified.

exact command evidence
- `cargo fmt --all -- --check` -> exit 0
- `cargo check --release` -> exit 0
- `cargo build --release` -> exit 0
- `cargo test --release` -> exit 0
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only` -> exit 0
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores` -> exit 0
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` -> exit 0
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` -> exit 1
- `rustfmt --edition 2024 --check .omo/evidence/task-58-coloring-probe.rs` -> exit 0
- `rustc --edition=2024 -A dead_code .omo/evidence/task-58-coloring-probe.rs -o /tmp/task-58-coloring-probe-verify` -> exit 0
- `/tmp/task-58-coloring-probe-verify` -> exit 0
- `cargo clippy --all-targets --all-features -- -D warnings` -> exit 101
- `git diff --check -- src/codegen/regalloc/mod.rs` -> exit 0
- `git diff --check --cached` -> exit 0
- `git status --short -- src/codegen/regalloc/mod.rs src/codegen/regalloc/color.rs .omo/evidence/task-58-coloring-implementation.txt .omo/evidence/task-58-coloring-probe.rs` -> exit 0
- `pure LOC check task 58 files` -> exit 0
- `scoped unsafe/debug/escape-hatch scan` -> exit 0

chapter 20 failure detail
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` -> exit 1. Output summary: `Ran 66 tests ... FAILED (failures=2, errors=46)`. Representative failures: missing chapter_20 helper assembly files such as `helper_libs/wrapper_linux.s`/`clobber_xmm_regs_linux.s`; behavioral failures `mixed_type_funcall_generates_args` (expected 0, got 255; `Expected s1.l to be -50, found ...`) and `type_conversion_interference` (expected `4294967295`, found `18446744073709551615`).

durable probe evidence
```text
===== COMMAND: /tmp/task-58-coloring-probe-verify =====
cwd: /home/mei/projects/rustcc
start: 2026-07-09T02:23:34-04:00
{
    "gp_allocatable": "[AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15]",
    "hardreg_conflict": "{Pseudo(\"c\"): Some(BX)}",
    "small_graph": "{Pseudo(\"a\"): Some(BX), Pseudo(\"b\"): Some(AX)}",
    "spill_candidate": "{Pseudo(\"pressure\"): None}",
    "xmm_allocatable": "[XMM(0), XMM(1), XMM(2), XMM(3), XMM(4), XMM(5), XMM(6), XMM(7), XMM(8), XMM(9), XMM(10), XMM(11), XMM(12), XMM(13)]",
}
exit: 0
end: 2026-07-09T02:23:34-04:00
```

additional production-types cross-check
```text
===== COMMAND: rustfmt --edition 2024 /tmp/task58_extra_probe.rs =====
exit: 0
===== COMMAND: rustc --edition=2024 -A dead_code /tmp/task58_extra_probe.rs -o /tmp/task58_extra_probe && /tmp/task58_extra_probe =====
extra_probe_ok gp=[AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15] xmm=[XMM(0), XMM(1), XMM(2), XMM(3), XMM(4), XMM(5), XMM(6), XMM(7), XMM(8), XMM(9), XMM(10), XMM(11), XMM(12), XMM(13)] caller=SelectResult { assignments: {Pseudo("caller"): Some(AX)}, used_callee_saved_regs: {} } callee=SelectResult { assignments: {Pseudo("callee"): Some(BX)}, used_callee_saved_regs: {BX} }
exit: 0
```

scoped status / hygiene evidence
```text
===== COMMAND: git status --short -- src/codegen/regalloc/mod.rs src/codegen/regalloc/color.rs .omo/evidence/task-58-coloring-implementation.txt .omo/evidence/task-58-coloring-probe.rs =====
cwd: /home/mei/projects/rustcc
start: 2026-07-09T02:23:35-04:00
 M src/codegen/regalloc/mod.rs
?? .omo/evidence/task-58-coloring-implementation.txt
?? .omo/evidence/task-58-coloring-probe.rs
?? src/codegen/regalloc/color.rs
exit: 0
end: 2026-07-09T02:23:35-04:00

===== COMMAND: pure LOC check task 58 files =====
cwd: /home/mei/projects/rustcc
start: 2026-07-09T02:23:35-04:00
src/codegen/regalloc/color.rs 85
src/codegen/regalloc/mod.rs 56
.omo/evidence/task-58-coloring-probe.rs 220
exit: 0
end: 2026-07-09T02:23:36-04:00

===== COMMAND: scoped unsafe/debug/escape-hatch scan =====
cwd: /home/mei/projects/rustcc
start: 2026-07-09T02:23:36-04:00
src/codegen/regalloc/mod.rs:75:    unimplemented!("ch.20 regalloc wired in wave 21")
exit: 0
end: 2026-07-09T02:23:36-04:00
```

OCaml parity evidence
- OCaml hardreg setup: `GP.all_hardregs = [AX; BX; CX; DX; DI; SI; R8; R9; R12; R13; R14; R15]`, `caller_saved_regs = [AX; CX; DX; DI; SI; R8; R9]` (`nqcc2/lib/backend/regalloc.ml:607-610`).
- OCaml coloring rule: hardreg nodes that are not caller-saved choose max available color, otherwise min (`regalloc.ml:532-538`), then colors are mapped back to hardregs (`546-563`).
- Fresh local simulation of those lines produced `ocaml_color_to_reg {0: 'R9', 1: 'R8', 2: 'SI', 3: 'DI', 4: 'DX', 5: 'CX', 6: 'AX', 7: 'BX', 8: 'R12', 9: 'R13', 10: 'R14', 11: 'R15'}`; Rust precolor order maps color 0 -> `AX`.

remove-ai-slops / programming direct pass
- Excessive/useless/deletion-only tests: no deletion-only tests, but the durable probe is implementation-shaped and overfit because it stubs most dependencies and misses caller-saved/callee-saved assertions.
- Tautological / implementation-mirroring risk: present. The probe mostly asserts a hand-built graph around the new `select` helper rather than a real compiler/user-visible behavior.
- Missing tests: blocking. No real Rust unit test or chapter gate proves select behavior.
- Production slop scan: no `unsafe`, `unwrap`, `expect`, `dbg!`, `println!`, or `eprintln!` in `src/codegen/regalloc/color.rs`; scoped scan found `unimplemented!` in `src/codegen/regalloc/mod.rs:75`.
- LOC: `src/codegen/regalloc/color.rs` 85 pure LOC, `mod.rs` 56, durable probe 220; under 250.
- Parameter bloat: `color_node` 5 params, `remove_neighbor_color` 4 params.
- Reserved regs: production `RegisterClass::Gp.all_hardregs()` excludes `R10/R11/SP/BP`; `Xmm` uses `0..=13`, excluding `XMM14/XMM15`.
- Spill marker: source `available.into_iter().next().and_then(...)` returns `None` when all colors are unavailable; durable probe `spill_candidate` confirms `Pseudo("pressure"): None`.

spawned code-review report coverage
- Spawned read-only code-review result: `019f4584-3f8f-7962-a66c-e60d02fe110d` returned `recommendation REJECT`.
- It independently covered programming/remove-ai-slops perspectives and found the same blockers: chapter 20 no-coalescing gate unmet, `allocate` still `unimplemented!`, OCaml color-map parity mismatch, and overfit/manual-only probe evidence.

ultraqa/adversarial notes
- Stale-success defense: did not trust implementation prose; reran gates and probe from current working tree.
- Official-gate defense: ran the Task 58 chapter 20 no-coalescing gate despite implementation evidence claiming it was out of scope; it failed.
- Probe-overfit defense: inspected the probe source and found copied stubs/absolute path include; added a temporary production-types cross-check, which still does not replace a real test.
- OCaml-parity defense: compared Rust color mapping against OCaml hardreg color assignment and color-to-register reconstruction.
- Scope-fidelity defense: confirmed no spill rewrite/coalescing implementation was added, but that means the plan acceptance cannot be met yet.
- Dirty-worktree defense: scoped status shows only Task 58 code/evidence paths in this gate scope (`mod.rs` modified; `color.rs`, implementation evidence, and probe untracked) plus unrelated historical evidence outside scope.
- Cleanup: temporary `/tmp` probes/logs/scripts removed.

cleanup evidence
- Cleanup command: `rm -f /tmp/task58_verify.sh /tmp/task58_verify_commands.log /tmp/task58_extra_probe /tmp/task58_extra_probe.rs /tmp/task58_extra_probe.out /tmp/task-58-coloring-probe-verify /tmp/task58_probe_* /tmp/task-58-coloring-probe /tmp/task58_verify_extra.rs`
- Cleanup verification: `find /tmp -maxdepth 1 ( -name task58* -o -name task-58* ) -print` equivalent found: no matches

exact evidence gaps
- No Cargo-integrated tests for `select`/`ColorMap`/`SelectResult`.
- No durable probe that compiles production `RegisterClass` and production graph/simplify together.
- No passing chapter 20 no-coalescing official gate.
- No pre-existing Task 58 code review/manual QA artifact; spawned read-only review result was REJECT.
- No integration from `select` into `allocate`; `allocate` remains an unimplemented placeholder.

Final verdict: NEEDS-FIX
