VERDICT: REJECT

# Task 58 W21-T3/no-coalescing final code review after gate fix

- **codeQualityStatus:** BLOCK
- **recommendation:** REQUEST_CHANGES
- **reportPath:** `.omo/evidence/task-58-coloring-code-review-2.md`
- **Review mode:** read-only; wrote only this report.
- **Skill perspectives:** Loaded and applied `omo:remove-ai-slops` and `omo:programming` (plus Rust reference and code-smells reference). The diff still violates the programming/remove-ai-slops perspectives because production behavior is gated by a test-suite path string and the probe/evidence overstates OCaml parity.

## Summary

Positive evidence is real: the Chapter 20 no-coalescing gate now passes, the durable probe compiles production regalloc sources and checks mapping/callee-saved/reserved/spill behavior, the Chapter 18/19 regression gates pass, the restored `.s` helper fixtures match upstream official files and are ignored by `*.s`, and I found no coalescing implementation, new dependency, or unsafe block.

I am still rejecting because two acceptance-level issues remain:

1. `compile` uses `source_path_hint.contains("chapter_20")` to enable register allocation even when coalescing is enabled. That is production logic keyed to a test directory/chapter path, i.e. a compiler/test bridge, and it is not needed for the required `--no-coalescing` gate.
2. The current probe labels its mapping `ocaml_color_mapping`, but local OCaml reference `nqcc2/lib/backend/regalloc.ml` excludes `R11` from `GP.all_hardregs`; Rust now includes `R11` and reports color 0 as `R11`. The select algorithm shape is closer than before, but exact OCaml select/color parity is not established as claimed.

## Findings by severity

### CRITICAL

None.

### HIGH

#### HIGH-1: Production allocation is gated on the source file path containing `chapter_20`

- **Location:** `src/compiler.rs:131-135`
- **Code:** `should_allocate` is true when `!options.regalloc_options.coalescing_enabled` **or** when `source_path_hint` contains `"chapter_20"`.
- **Why this blocks:** Task criteria include no compiler-phase test/bridge. A compiler pass should be selected by explicit compiler options/capabilities, not by whether the input path happens to live under a test-suite chapter directory. This makes normal user behavior path-dependent: a Chapter 20-like program outside `tests/tests/chapter_20/...` follows a different default path than the same source inside that directory.
- **Scope impact:** The required no-coalescing acceptance does not need this branch; `--no-coalescing` already makes `coalescing_enabled == false`. The path branch appears to preserve/force chapter-scoped test behavior rather than encode compiler semantics.

#### HIGH-2: OCaml parity claim is not exact after adding `R11` to allocatable GP registers

- **Locations:**
  - Rust register set: `src/codegen/regalloc/types.rs:54-68`, `src/codegen/regalloc/types.rs:75-84`
  - Probe expected mapping: `.omo/evidence/task-58-coloring-probe.rs:116-118`
  - OCaml reference: `nqcc2/lib/backend/regalloc.ml:607-610`; color selection `nqcc2/lib/backend/regalloc.ml:532-538`
- **Evidence:** Local OCaml `GP.all_hardregs` is `[AX; BX; CX; DX; DI; SI; R8; R9; R12; R13; R14; R15]` and caller-saved is `[AX; CX; DX; DI; SI; R8; R9]`. Simulating that OCaml selection gives `{0: R9, 1: R8, 2: SI, 3: DI, 4: DX, 5: CX, 6: AX, 7: BX, 8: R12, 9: R13, 10: R14, 11: R15}`.
- **Current Rust/probe output:** `{0: R11, 1: R9, 2: R8, 3: SI, 4: DI, 5: DX, 6: CX, 7: AX, 8: BX, 9: R12, 10: R13, 11: R14, 12: R15}`.
- **Why this blocks:** The task asks to confirm select/color OCaml parity. The algorithm now applies the OCaml min/max color policy to Rust's enlarged register set, but that is a deliberate divergence from the local OCaml register class. Either justify/document the R11 divergence as intended non-parity, or restore exact reference parity and solve the resulting pressure another way.

### MEDIUM

#### MEDIUM-1: New regalloc rewrite code adds a Clippy `-D warnings` finding

- **Location:** `src/codegen/regalloc/rewrite.rs:17-20`
- **Evidence:** `cargo clippy --all-targets --all-features -- -D warnings` exits 101. The project has many pre-existing Clippy failures, but this new file adds `clippy::collapsible_if` in the changed scope.
- **Risk:** Not a functional blocker while Clippy is already red, but it adds a new warning to an already failing static-analysis gate.

#### MEDIUM-2: Tightly scoped but near-limit modules need watch status

- `src/codegen/regalloc/allocate.rs` is 240 pure LOC and `.omo/evidence/task-58-coloring-probe.rs` is 232 pure LOC. Both are under the 250 pure-LOC ceiling but in the programming warning band.
- `src/codegen/codegen.rs` and `src/codegen/emit.rs` remain oversized pre-existing modules; this change did not materially worsen them, but future edits should split by responsibility.

### LOW

#### LOW-1: Durable probe is a standalone artifact, not integrated regression coverage

- `.omo/evidence/task-58-coloring-probe.rs` compiles production `assembly.rs`, `types.rs`, `operands.rs`, `graph.rs`, `simplify.rs`, and `color.rs` via absolute `#[path]` attributes and asserts key selection behavior.
- This is acceptable for the requested evidence artifact, but it is not a Cargo-integrated regression test. Future changes can bypass it unless reviewers keep running the artifact manually.

## Acceptance criteria review

- **Chapter 20 no-coalescing gate:** PASS by fresh run (`Ran 66 tests ... OK`, exit 0).
- **Select/color probe:** PASS mechanically; it compiles production source files and checks mapping/callee-saved/reserved/spill behavior. However, its `ocaml_color_mapping` expectation diverges from local OCaml because Rust now includes `R11`.
- **No-coalescing allocation path:** Mostly reasonable: GP then XMM allocation uses liveness -> interference -> simplify -> select; uncolored pseudos remain for stack placement. `rg -n "coalesc|coalesce"` found no coalescing implementation, only comments/option gating.
- **Restored `.s` fixtures:** Legitimate. They match upstream `nlsandler/writing-a-c-compiler-tests` raw files by SHA-256/diff and are ignored by `.gitignore:17:*.s`, so they must be `git add -f`'d later if committing.
- **Chapter 18/19 regressions:** PASS by fresh runs.
- **Scope hygiene:** No Cargo dependency changes, no production `unsafe`, no debug print macros in changed production files. Blocking hygiene issue is the path-based chapter gate.

## Exact command evidence

```text
$ cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
exit: 0

$ cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
exit: 0

$ cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
exit: 0

$ cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md
exit: 0

$ cat .omo/evidence/task-58-coloring-adversarial-verify.txt
exit: 0

$ cat .omo/evidence/task-58-coloring-fix.txt
exit: 0

$ cat .omo/evidence/task-58-no-coalescing-gate-fix.txt
exit: 0 (tool output truncated, so I also inspected command headings and tail)

$ git status --short --branch
## main...origin/main [ahead 50]
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
?? src/codegen/regalloc/allocate.rs
?? src/codegen/regalloc/color.rs
?? src/codegen/regalloc/rewrite.rs
... evidence files omitted here ...
exit: 0

$ git diff --stat
10 tracked files changed, 152 insertions(+), 63 deletions(-)
exit: 0

$ git diff --no-index -- /dev/null src/codegen/regalloc/allocate.rs
exit: 1 (new-file diff inspected)

$ git diff --no-index -- /dev/null src/codegen/regalloc/color.rs
exit: 1 (new-file diff inspected)

$ git diff --no-index -- /dev/null src/codegen/regalloc/rewrite.rs
exit: 1 (new-file diff inspected)

$ nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '470,565p;607,637p'
exit: 0

$ nl -ba nqcc2/lib/assembly.ml | sed -n '1,60p'
exit: 0

$ nl -ba .omo/evidence/task-58-coloring-probe.rs | sed -n '1,290p'
exit: 0

$ python3 - <<'PY'
# simulate OCaml hardreg coloring from local nqcc2 lines 607-610 and 532-538
...
PY
{0: 'R9', 1: 'R8', 2: 'SI', 3: 'DI', 4: 'DX', 5: 'CX', 6: 'AX', 7: 'BX', 8: 'R12', 9: 'R13', 10: 'R14', 11: 'R15'}
exit: 0

$ curl -fsSL https://raw.githubusercontent.com/nlsandler/writing-a-c-compiler-tests/main/tests/chapter_20/helper_libs/wrapper_linux.s -o /tmp/upstream-wrapper_linux.s
$ diff -u /tmp/upstream-wrapper_linux.s tests/tests/chapter_20/helper_libs/wrapper_linux.s
exit: 0; sha256 local/upstream 8777f471d8300d20f6c8c98644cb4db4deced1d3f0652488f7c5e98969b54c23

$ curl -fsSL https://raw.githubusercontent.com/nlsandler/writing-a-c-compiler-tests/main/tests/chapter_20/helper_libs/clobber_xmm_regs_linux.s -o /tmp/upstream-clobber_xmm_regs_linux.s
$ diff -u /tmp/upstream-clobber_xmm_regs_linux.s tests/tests/chapter_20/helper_libs/clobber_xmm_regs_linux.s
exit: 0; sha256 local/upstream d81f7dba0b7bd1694e94c8211e1d344cbfc3356f58e1259901f0e0ca65796bef

$ curl -fsSL https://raw.githubusercontent.com/nlsandler/writing-a-c-compiler-tests/main/tests/chapter_20/helper_libs/alignment_check_wrapper_linux.s -o /tmp/upstream-alignment_check_wrapper_linux.s
$ diff -u /tmp/upstream-alignment_check_wrapper_linux.s tests/tests/chapter_20/helper_libs/alignment_check_wrapper_linux.s
exit: 0; sha256 local/upstream b6187436655fceb2b7999e863b1c172119e858a905eeeed7b1c11979e46bbb9e

$ git check-ignore -v tests/tests/chapter_20/helper_libs/*.s
.gitignore:17:*.s tests/tests/chapter_20/helper_libs/alignment_check_wrapper_linux.s
.gitignore:17:*.s tests/tests/chapter_20/helper_libs/clobber_xmm_regs_linux.s
.gitignore:17:*.s tests/tests/chapter_20/helper_libs/wrapper_linux.s
exit: 0

$ cargo fmt --all -- --check
exit: 0

$ cargo check --release
Finished `release` profile [optimized] target(s) in 0.03s
exit: 0

$ cargo build --release
Finished `release` profile [optimized] target(s) in 0.01s
exit: 0

$ cargo test --release
10 passed; 0 failed
exit: 0

$ cargo clippy --all-targets --all-features -- -D warnings
exit: 101
Relevant changed-scope finding: src/codegen/regalloc/rewrite.rs:17:5 clippy::collapsible_if.
Many other failures appear pre-existing outside this task scope.

$ ./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
Ran 66 tests in 3.220s
OK
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only
Ran 120 tests in 2.893s
OK
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores
Ran 27 tests in 0.648s
OK
exit: 0

$ ./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union
Ran 286 tests in 5.275s
OK
exit: 0

$ rustfmt --edition 2024 --check .omo/evidence/task-58-coloring-probe.rs
exit: 0

$ rustc --edition=2024 -A dead_code .omo/evidence/task-58-coloring-probe.rs -o /tmp/task-58-coloring-probe-review
exit: 0

$ /tmp/task-58-coloring-probe-review
{
    "callee_saved": "{BX}",
    "hardreg_conflict": "Some(R11)",
    "ocaml_color_mapping": "{0: R11, 1: R9, 2: R8, 3: SI, 4: DI, 5: DX, 6: CX, 7: AX, 8: BX, 9: R12, 10: R13, 11: R14, 12: R15}",
    "reserved": "{\"gp\": \"[AX, BX, CX, DX, DI, SI, R8, R9, R11, R12, R13, R14, R15]\", \"xmm\": \"[XMM(0), ..., XMM(13)]\"}",
    "small_conflict": "{Pseudo(\"a\"): Some(R9), Pseudo(\"b\"): Some(R11)}",
    "spill_marker": "None",
}
exit: 0

$ pure LOC/hygiene scoped
src/codegen/codegen/copy_prop_support.rs pure_loc=104 physical=115
src/ir/copy_propagation/rewrite_support.rs pure_loc=239 physical=247
src/pipeline.rs pure_loc=108 physical=182
src/compiler.rs pure_loc=179 physical=241
src/codegen/emit.rs pure_loc=504 physical=603
src/codegen/regalloc/allocate.rs pure_loc=240 physical=258
src/codegen/codegen.rs pure_loc=2006 physical=2231
src/codegen/fixup.rs pure_loc=235 physical=287
src/codegen/regalloc/types.rs pure_loc=96 physical=114
src/codegen/regalloc/mod.rs pure_loc=59 physical=80
src/codegen/regalloc/operands.rs pure_loc=152 physical=163
src/codegen/regalloc/color.rs pure_loc=118 physical=133
src/codegen/regalloc/rewrite.rs pure_loc=109 physical=113
.omo/evidence/task-58-coloring-probe.rs pure_loc=232 physical=265
scan found only existing `unwrap()` calls in `src/compiler.rs` tests; no production unsafe/debug/unimplemented hits in changed regalloc source.
exit: 0

$ git diff --check
exit: 0

$ rg -n "coalesc|coalesce" src/codegen/regalloc src/compiler.rs src/pipeline.rs
src/compiler.rs:131:    let should_allocate = !options.regalloc_options.coalescing_enabled
src/codegen/regalloc/mod.rs:1:// Mirrors nqcc2/lib/backend/regalloc.ml (651 LOC; uses Briggs/George coalescing).
src/codegen/regalloc/mod.rs:4:// simplification, select/color, and no-coalescing allocation are wired here.
exit: 0

$ git diff -- Cargo.toml Cargo.lock; echo exit:$?
exit:0
```

## Blockers before approval

1. Remove or redesign the `source_path_hint.contains("chapter_20")` production gate in `src/compiler.rs:131-135`. The allocation/coalescing decision must not depend on test directory names.
2. Resolve the OCaml parity claim: either make Rust's GP hardreg set/color mapping match local OCaml `nqcc2` exactly, or explicitly document and justify the `R11` divergence and stop presenting the probe output as exact `ocaml_color_mapping` parity.
3. Clean the new Clippy warning in `src/codegen/regalloc/rewrite.rs:17-20` if this branch is expected to keep `cargo clippy -D warnings` from regressing.
