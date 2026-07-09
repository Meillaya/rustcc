# Task 58 final fix code quality review

**Review date:** 2026-07-09
**Workspace:** `/home/mei/projects/rustcc`
**Mode:** read-only code quality review; production files were not edited.
**Goal reviewed:** Task58 / W21-T3 final fix for Chapter 20 no-coalescing coloring/select path and prior blockers.

## Verdict

**codeQualityStatus:** BLOCK
**recommendation:** REQUEST_CHANGES

The official Task58 gates now pass in this dirty workspace, and the prior R10/R11/register-class and source-path bridge blockers are fixed. However, the final fix is not safe to approve: a fresh adversarial no-coalescing program segfaults because `scratch.rs` rewrites R9 pointer-address scratch to R10, and later `replace_pseudos` uses R10 as its own memory-to-memory split scratch, clobbering the pointer before a spilled source store.

## Skill-perspective check

- `omo:remove-ai-slops` loaded and applied as a review lens. Result: the production diff still contains a critical hidden-cost/over-defensive scratch workaround whose interaction is not locked by tests; current tests/probe miss a real no-coalescing store-through-pointer spill case. No deletion-only tests found.
- `omo:programming` loaded with Rust/Python references and code-smell reference. Result: the diff violates the programming perspective on correctness and test shape for the scratch/allocation boundary. `allocate.rs` (242 pure LOC) and the probe (231 pure LOC) satisfy the user’s preferred <250 pure LOC constraint, but `tests/test_framework/runner.py` remains oversized at 421 pure LOC (pre-existing, touched by a minimal 4-line diff; noted as non-blocking scope risk).

## Findings

### CRITICAL

1. **No-coalescing can generate a self-clobbering indirect store and segfault.**
   - **Files/lines:** `src/codegen/regalloc/scratch.rs:21-36`, `src/codegen/regalloc/scratch.rs:39-75`, interacting with existing `src/codegen/replace_pseudos.rs:94-110`; no-coalescing path is enabled before pseudo replacement in `src/compiler.rs:123-130`.
   - **Issue:** `use_reserved_address_scratch` rewrites `movq <ptr>, %r9` + `mov <src>, 0(%r9)` into an R10-address pair. If `<src>` later remains spilled, `replace_pseudos` translates `movq Stack, Memory(R10,0)` using R10 as its split scratch, producing `movq stack, %r10; movq %r10, 0(%r10)`. The second instruction stores through the source value, not the pointer.
   - **Reproduction evidence:**
     ```text
     /tmp/task58_store_spill.c compiled with --no-coalescing -S:
       87  movq %r8, %r10        # pointer copied to R10
       88  movq -16(%rbp), %r10  # spilled store source clobbers pointer
       89  movq %r10, 0(%r10)    # writes through clobbered pointer
     gcc /tmp/task58_store_spill.s -o /tmp/task58_store_spill && /tmp/task58_store_spill
       -> Segmentation fault, exit 139
     Same program without --no-coalescing exits 146 as expected.
     ```
   - **Why this blocks:** This is a real user-visible miscompile/crash in the Task58 no-coalescing path, not a style issue. The passing official gate and probe do not cover this spill/store interaction.

### HIGH

1. **Chapter 20 gate reproducibility still depends on ignored, untracked helper assembly fixtures.**
   - **Files/lines:** `.gitignore:17` (`*.s`), ignored files under `tests/tests/chapter_20/helper_libs/`.
   - **Issue:** `git status --short --ignored -- tests/tests/chapter_20` reports:
     ```text
     !! tests/tests/chapter_20/helper_libs/alignment_check_wrapper_linux.s
     !! tests/tests/chapter_20/helper_libs/clobber_xmm_regs_linux.s
     !! tests/tests/chapter_20/helper_libs/wrapper_linux.s
     ```
     These files are not in `git diff` / `git ls-files`, yet prior Task58 gate failures showed the chapter 20 harness needs them.
   - **Why this blocks:** The green `--chapter 20 --latest-only --no-coalescing` result is not reproducible from the reviewed diff alone unless these ignored fixtures are force-added or otherwise made durable.

### MEDIUM

1. **Task58 probe is useful but not sufficient for allocation-pipeline safety.**
   - **File/lines:** `.omo/evidence/task-58-coloring-probe.rs:116-248`.
   - **Issue:** The probe now compiles production `types.rs`, `graph.rs`, `operands.rs`, `simplify.rs`, and `color.rs`, and it correctly checks OCaml GP color mapping and R10/R11 exclusion. It does not exercise `allocate.rs`, `scratch.rs`, `replace_pseudos`, or final emitted assembly, so it missed the CRITICAL store-spill crash.
   - **Impact:** False confidence risk for future regressions in no-coalescing integration; add an integration-style regression that compiles and runs a pressure/store-through-pointer case.

2. **Probe is path-local.**
   - **File/lines:** `.omo/evidence/task-58-coloring-probe.rs:5`, `:54-63`.
   - **Issue:** The `#[path = "/home/mei/projects/rustcc/..."]` includes make the evidence artifact tied to this absolute checkout path. It is still valid for this review run, but less durable than a repo-relative harness/script.

### LOW

1. **Size watch: touched runner remains oversized; allocation file is near the ceiling.**
   - **Files:** `tests/test_framework/runner.py` is 421 pure LOC; `src/codegen/regalloc/allocate.rs` is 242 pure LOC; `.omo/evidence/task-58-coloring-probe.rs` is 231 pure LOC.
   - **Impact:** `allocate.rs` and the probe meet the user’s preferred under-250 constraint, but `allocate.rs` has little headroom. The runner oversize is pre-existing and the diff is a minimal forwarding fix, so this is not a Task58 blocker.

## Checks that passed

Commands run from `/home/mei/projects/rustcc`:

```text
git diff --check                                 PASS
cargo fmt --all -- --check                       PASS
cargo check --release                            PASS
cargo test --release                             PASS (10 tests)
cargo build --release                            PASS
cargo clippy --all-targets --all-features -- -A warnings  PASS
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing  PASS (66 tests)
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only                 PASS (120 tests)
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores  PASS (27 tests)
./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union          PASS (286 tests; known assembler truncation warnings)
rustfmt --edition 2024 --check .omo/evidence/task-58-coloring-probe.rs                   PASS
rustc --edition=2024 -A dead_code .omo/evidence/task-58-coloring-probe.rs && probe run    PASS
```

`cargo clippy --all-targets --all-features -- -D warnings` fails on pre-existing repository-wide warnings and at least one changed-area warning; the existing project evidence commonly uses `-A warnings`, which passed. This does not override the CRITICAL runtime blocker.

## Prior blocker status

- **R10/R11 allocatable:** fixed in current `src/codegen/regalloc/types.rs:52-105`; GP hardregs are `[AX, BX, CX, DX, DI, SI, R8, R9, R12, R13, R14, R15]`; caller-saved excludes R10/R11; XMM excludes XMM14/XMM15. Matches `nqcc2/lib/backend/regalloc.ml:607-637`.
- **OCaml color parity:** current probe output matches OCaml-derived mapping `{0:R9, 1:R8, 2:SI, 3:DI, 4:DX, 5:CX, 6:AX, 7:BX, 8:R12, 9:R13, 10:R14, 11:R15}`.
- **Source-path/test-name bridge:** fixed; `rg source_path_hint|source_path` found no remaining compiler bridge.
- **Runner `--no-coalescing` forwarding:** fixed in `tests/test_framework/runner.py:468-472`; chapter 20 no-coalescing gate passes in this workspace.
- **allocate.rs/probe LOC:** acceptable under the requested threshold/preference.

## Blockers before approval

1. Fix the R10 self-clobber in no-coalescing store-through-pointer cases and lock it with a regression that fails on the generated `movq stack, %r10; movq %r10, 0(%r10)` pattern / runtime segfault.
2. Make the required chapter 20 helper `.s` fixtures durable in the reviewed diff (or otherwise make the gate reproducible from a clean checkout).

## Non-blocking notes

- Consider replacing the absolute-path probe with a repo-relative script/harness.
- Consider splitting or planning a split for `allocate.rs` before the next substantive addition; it is currently 242 pure LOC.
