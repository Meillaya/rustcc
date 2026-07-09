VERDICT: NEEDS-FIX
recommendation: REJECT

Task: Final adversarial gate for Task 58 after no-coalescing gate fix
Date: 2026-07-09
Workspace: /home/mei/projects/rustcc
Mode: read-only verification; no code/plan/docs/Boulder/git edits made by this gate.

originalIntent
- Complete Task 58 / W21-T3 Chapter 20 coloring + select phase.
- Resolve the prior blockers: chapter 20 no-coalescing gate, no-coalescing allocation wiring, OCaml color mapping, durable production-source probe, legitimate fixtures, official regressions, and scope/hygiene.
- Preserve previous W21 register-class/liveness invariants unless intentionally and correctly superseded.

desiredOutcome
- User can mark Task 58 complete only if current source, durable evidence, official gates, fixture provenance, code-review coverage, and slop/overfit review all support completion.

userOutcomeReview
- NEEDS-FIX. The fresh official gates now pass, including `--chapter 20 --latest-only --no-coalescing`, and allocation is wired into the current compiler path.
- However, the fix regressed the previously confirmed Task 56 / OCaml register-class contract by making `R11` allocatable and caller-saved in `RegisterClass::Gp`. Local OCaml reference `nqcc2/lib/backend/regalloc.ml` excludes both `R10` and `R11` from GP `all_hardregs` and `caller_saved_regs`; prior Task 56 gate evidence explicitly confirmed R10/R11 exclusion as a required invariant.
- The durable probe now passes because it hardcodes the new R11-inclusive mapping `{0: R11, ...}`, not because it proves parity with the local OCaml reference. Direct parity simulation shows current Rust color mapping does not match local OCaml color mapping.
- The available Task 58 code-review artifact is stale: it predates the no-coalescing fix, did not inspect `allocate.rs`, `rewrite.rs`, or the broader codegen/fixup/compiler changes, and explicitly says it did not run the chapter 20 no-coalescing gate. This fails the final-gate requirement that the code-review report cover the same skill-perspective and slop/overfit criteria for the shipped diff.

checked artifact paths
- `.omo/evidence/task-58-coloring-implementation.txt`
- `.omo/evidence/task-58-coloring-code-review.md`
- `.omo/evidence/task-58-coloring-gate-review.md`
- `.omo/evidence/task-58-coloring-fix.txt`
- `.omo/evidence/task-58-no-coalescing-gate-fix.txt`
- `.omo/evidence/task-58-coloring-probe.rs`
- `.omo/evidence/task-56-liveness-adversarial-verify-2.txt`
- `.omo/evidence/task-56-liveness-code-review-2.md`
- `.omo/plans/c-compiler-rust.md`
- `nqcc2/lib/backend/regalloc.ml`
- `src/codegen/regalloc/types.rs`
- `src/codegen/regalloc/color.rs`
- `src/codegen/regalloc/allocate.rs`
- `src/codegen/regalloc/rewrite.rs`
- `src/codegen/regalloc/mod.rs`
- `src/compiler.rs`
- `src/pipeline.rs`
- `src/codegen/fixup.rs`
- `src/codegen/codegen.rs`
- `src/codegen/emit.rs`

blockers
1. OCaml/register-class parity regression: current `src/codegen/regalloc/types.rs` includes `Reg::R11` in GP `all_hardregs`, GP `caller_saved_regs`, and `contains`, but local OCaml reference excludes R11. This also contradicts prior confirmed Task 56 evidence that R10/R11/XMM14/XMM15 are reserved/excluded.
2. Color mapping is not OCaml-parity: corrected local simulation gives OCaml `color_to_reg={0:R9,1:R8,2:SI,3:DI,4:DX,5:CX,6:AX,7:BX,8:R12,9:R13,10:R14,11:R15}` while current Rust/probe gives `{0:R11,1:R9,2:R8,3:SI,4:DI,5:DX,6:CX,7:AX,8:BX,9:R12,10:R13,11:R14,12:R15}`.
3. Durable probe is overfit to the regression: `.omo/evidence/task-58-coloring-probe.rs` compiles production source for select/color support, but its expected OCaml mapping includes R11 and therefore does not validate the local OCaml reference contract.
4. Code-review coverage is stale/unsupported for the shipped diff: `.omo/evidence/task-58-coloring-code-review.md` predates `.omo/evidence/task-58-no-coalescing-gate-fix.txt`, does not cover the current broad diff, and says no-coalescing was not run because allocate was a placeholder.
5. Fixture durability risk: the three chapter 20 `.s` helper fixtures match upstream, but they are ignored/untracked (`!!`) due `.gitignore: *.s`; if Task 58 completion is later committed without `git add -f`, the chapter 20 gate will not be reproducible from the commit.
6. Hygiene watch: changed legacy files `src/codegen/codegen.rs` and `src/codegen/emit.rs` remain over the 250 pure-LOC programming ceiling. This appears pre-existing and not the primary blocker, but it is still a review risk in the current broad diff.

exact command evidence

$ git rev-parse HEAD
7a4ae7434feb02ae477a5efbaa380b0c495bd03a
exit: 0

$ git status --short
 M src/codegen/codegen.rs
 M src/codegen/codegen/copy_prop_support.rs
 M src/codegen/emit.rs
 M src/codegen/fixup.rs
 M src/codegen/regalloc/mod.rs
 M src/codegen/regalloc/operands.rs
 M src/codegen/regalloc/types.rs
 M src/compiler.rs
 M src/ir/copy_propagation/rewrite_support.rs
 M src/pipeline.rs
?? .omo/evidence/task-58-coloring-adversarial-verify.txt
?? .omo/evidence/task-58-coloring-code-review.md
?? .omo/evidence/task-58-coloring-fix.txt
?? .omo/evidence/task-58-coloring-gate-review.md
?? .omo/evidence/task-58-coloring-implementation.txt
?? .omo/evidence/task-58-coloring-probe.rs
?? .omo/evidence/task-58-no-coalescing-gate-fix.txt
?? src/codegen/regalloc/allocate.rs
?? src/codegen/regalloc/color.rs
?? src/codegen/regalloc/rewrite.rs
exit: 0

$ git diff --check
exit: 0

$ cargo fmt --all -- --check
exit: 0

$ cargo check --release
    Finished `release` profile [optimized] target(s) in 0.05s
exit: 0

$ cargo build --release
    Finished `release` profile [optimized] target(s) in 0.01s
exit: 0

$ cargo test --release
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Doc-tests rustcc: ok
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
----------------------------------------------------------------------
Ran 66 tests in 3.184s

OK
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
----------------------------------------------------------------------
Ran 120 tests in 2.967s

OK
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
----------------------------------------------------------------------
Ran 27 tests in 0.621s

OK
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
----------------------------------------------------------------------
Ran 286 tests in 5.291s

OK
chapter_18 initializer assembler truncation warnings remained non-fatal.
exit: 0

$ rustfmt --edition 2024 --check .omo/evidence/task-58-coloring-probe.rs
rustfmt exit: 0
$ rustc --edition=2024 -A dead_code .omo/evidence/task-58-coloring-probe.rs -o /tmp/task-58-coloring-probe-gate2
rustc exit: 0
$ /tmp/task-58-coloring-probe-gate2
{
    "callee_saved": "{BX}",
    "hardreg_conflict": "Some(R11)",
    "ocaml_color_mapping": "{0: R11, 1: R9, 2: R8, 3: SI, 4: DI, 5: DX, 6: CX, 7: AX, 8: BX, 9: R12, 10: R13, 11: R14, 12: R15}",
    "reserved": "{\"gp\": \"[AX, BX, CX, DX, DI, SI, R8, R9, R11, R12, R13, R14, R15]\", \"xmm\": \"[XMM(0), XMM(1), XMM(2), XMM(3), XMM(4), XMM(5), XMM(6), XMM(7), XMM(8), XMM(9), XMM(10), XMM(11), XMM(12), XMM(13)]\"}",
    "small_conflict": "{Pseudo(\"a\"): Some(R9), Pseudo(\"b\"): Some(R11)}",
    "spill_marker": "None",
}
probe run exit: 0

$ cargo clippy --all-targets --all-features -- -A warnings
    Checking rustcc v0.0.1 (/home/mei/projects/rustcc)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.79s
exit: 0

$ python local OCaml/Rust parity probe
local_ocaml_gp_all_hardregs= ['AX', 'BX', 'CX', 'DX', 'DI', 'SI', 'R8', 'R9', 'R12', 'R13', 'R14', 'R15']
rust_gp_all_hardregs= ['AX', 'BX', 'CX', 'DX', 'DI', 'SI', 'R8', 'R9', 'R11', 'R12', 'R13', 'R14', 'R15']
gp_all_match= False
local_ocaml_gp_caller_saved= ['AX', 'CX', 'DX', 'DI', 'SI', 'R8', 'R9']
rust_gp_caller_saved= ['AX', 'CX', 'DX', 'DI', 'SI', 'R8', 'R9', 'R11']
gp_caller_match= False
local_ocaml_color_to_reg= {0: 'R9', 1: 'R8', 2: 'SI', 3: 'DI', 4: 'DX', 5: 'CX', 6: 'AX', 7: 'BX', 8: 'R12', 9: 'R13', 10: 'R14', 11: 'R15'}
rust_color_to_reg= {0: 'R11', 1: 'R9', 2: 'R8', 3: 'SI', 4: 'DI', 5: 'DX', 6: 'CX', 7: 'AX', 8: 'BX', 9: 'R12', 10: 'R13', 11: 'R14', 12: 'R15'}
color_map_match= False
exit: 0

$ nl -ba src/codegen/regalloc/types.rs | sed -n '51,108p'
52 pub fn all_hardregs(self) -> Vec<Reg> { ... Reg::R11 ... }
73 pub fn caller_saved_regs(self) -> Vec<Reg> { ... Reg::R11 ... }
89 pub fn contains(self, reg: &Reg) -> bool { ... Reg::R11 ... }
exit: 0

$ nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '607,635p'
609 let all_hardregs = [ AX; BX; CX; DX; DI; SI; R8; R9; R12; R13; R14; R15 ]
610 let caller_saved_regs = [ AX; CX; DX; DI; SI; R8; R9 ]
617-632 XMM0 through XMM13 only
exit: 0

$ grep prior Task 56 evidence for reserved register invariant
.omo/evidence/task-56-liveness-adversarial-verify-2.txt:13: RegisterClass now matches OCaml ... (R10/R11/XMM14/XMM15 excluded)
.omo/evidence/task-56-liveness-adversarial-verify-2.txt:204: PASS contains(): R10/R11/XMM14/XMM15 excluded
.omo/evidence/task-56-liveness-adversarial-verify-2.txt:208: PASS call GP caller-saved excludes scratch R10/R11
.omo/evidence/task-56-liveness-code-review-2.md:391: GP excludes `R10/R11`; XMM excludes `XMM14/XMM15`: **PASS**.
exit: 0

$ compare chapter20 .s fixtures to upstream nlsandler/writing-a-c-compiler-tests main
wrapper_linux.s local_sha=8777f471d8300d20f6c8c98644cb4db4deced1d3f0652488f7c5e98969b54c23 upstream_sha=8777f471d8300d20f6c8c98644cb4db4deced1d3f0652488f7c5e98969b54c23 upstream_cmp=MATCH
clobber_xmm_regs_linux.s local_sha=d81f7dba0b7bd1694e94c8211e1d344cbfc3356f58e1259901f0e0ca65796bef upstream_sha=d81f7dba0b7bd1694e94c8211e1d344cbfc3356f58e1259901f0e0ca65796bef upstream_cmp=MATCH
alignment_check_wrapper_linux.s local_sha=b6187436655fceb2b7999e863b1c172119e858a905eeeed7b1c11979e46bbb9e upstream_sha=b6187436655fceb2b7999e863b1c172119e858a905eeeed7b1c11979e46bbb9e upstream_cmp=MATCH
exit: 0

$ git status --short --ignored -- tests/tests/chapter_20/helper_libs
!! tests/tests/chapter_20/helper_libs/alignment_check_wrapper_linux.s
!! tests/tests/chapter_20/helper_libs/clobber_xmm_regs_linux.s
!! tests/tests/chapter_20/helper_libs/wrapper_linux.s
exit: 0

$ Task 58 code review artifact freshness/coverage probe
-rw-r--r-- .omo/evidence/task-58-coloring-code-review.md Jul 9 02:19
-rw-r--r-- .omo/evidence/task-58-no-coalescing-gate-fix.txt Jul 9 03:08
code review excerpt: "I did not run `--chapter 20 --latest-only --no-coalescing` ... `allocate` intentionally remains a placeholder in this slice."
current changed production paths include allocate.rs, rewrite.rs, codegen.rs, fixup.rs, compiler.rs, pipeline.rs, regalloc types/mod/operands/color.
exit: 0

$ all changed production-file LOC and hygiene scan
src/codegen/codegen.rs pure_loc=2006 physical=2231
src/codegen/emit.rs pure_loc=504 physical=603
src/codegen/fixup.rs pure_loc=235 physical=287
src/codegen/regalloc/allocate.rs pure_loc=240 physical=258
src/codegen/regalloc/color.rs pure_loc=118 physical=133
src/codegen/regalloc/rewrite.rs pure_loc=109 physical=113
.omo/evidence/task-58-coloring-probe.rs pure_loc=232 physical=265
scan output only found unwraps in existing `#[cfg(test)]` unit tests in `src/compiler.rs`; no unsafe/dbg/println/eprintln/TODO/FIXME/unimplemented/panic matches in production changed scope.
exit: 0

remove-ai-slops / programming direct pass
- Loaded `omo:remove-ai-slops` and applied overfit/slop lens directly.
- Loaded `omo:programming`, Rust README, and code-smells reference; applied Rust/source hygiene lens directly.
- Excessive/useless tests: no Cargo tests were added, but the durable probe is overfit because it names the wrong R11-inclusive expected mapping and therefore gives false confidence.
- Deletion-only/tautological tests: none added.
- Implementation mirroring: durable probe is low-level select/color API-focused, acceptable only if its reference expectations are correct; current expectations are not.
- Unnecessary production extraction/parsing/normalization: no clear new helper-only slop in allocate/color/rewrite, but broad codegen/fixup changes lack fresh code-review coverage.
- Parameter/size smells: new files are under 250 pure LOC; changed legacy `codegen.rs`/`emit.rs` remain oversized pre-existing modules.

ultraqa/adversarial notes
- Stale-success defense: reran gates and durable probe from current worktree instead of trusting fix evidence.
- Official-gate defense: chapter 20 no-coalescing, chapter 19 latest, chapter 19 DSE, chapter 18 union, cargo build/check/test all reran green.
- Probe-overfit defense: compared the probe's expected mapping against local OCaml source and prior Task 56 evidence; found R11 regression.
- Fixture defense: compared helper `.s` fixtures to upstream raw files; content matches, but git shows them ignored/untracked.
- Scope defense: current changed production paths are broader than the stale code review covered.
- Cleanup: `/tmp/task-58-coloring-probe-gate2` and `/tmp/task58_gate2.log` removed; `find /tmp -maxdepth 1 (task58_gate2* or task-58-coloring-probe-gate2*)` produced no matches.

exact evidence gaps
- No post-no-coalescing-fix code-review report covering the current broad diff and slop/overfit criteria.
- No correct durable probe or integrated test proving local OCaml GP hardreg/caller-saved parity after the R11 change.
- No explanation justifying why Task 58 may break Task 56's confirmed R10/R11 reserved invariant.
- Chapter 20 helper `.s` fixtures are legitimate but ignored/untracked, so reproducibility depends on force-adding them later.

Final verdict: NEEDS-FIX
