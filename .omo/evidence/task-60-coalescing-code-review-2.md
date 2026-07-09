# Task 60 Coalescing Code Review 2

Verdict: APPROVE
codeQualityStatus: WATCH
recommendation: APPROVE
reviewed_at: 2026-07-09
workspace: `/home/mei/projects/rustcc`
mode: read-only review; wrote only this artifact

## Scope Reviewed

Re-reviewed current uncommitted Task60 after the cleanup fix, including the required evidence:

- `.omo/evidence/task-60-coalescing-fix.txt`
- `.omo/evidence/task-60-coalescing-code-review.md`
- `.omo/evidence/task-60-coalescing-adversarial-verify.txt`

Inspected current source/diff for:

- `src/codegen/regalloc/{allocate,coalesce,division_copy,abi_liveness,graph,graph_pseudos,rewrite,mod}.rs`
- `src/codegen/{fixup,fixup/split,replace_pseudos,replace_pseudos/split,replace_pseudos/move_split,xmm}.rs`
- `src/codegen/codegen.rs`, `src/driver.rs`, `src/compiler.rs`, `src/pipeline.rs`
- `nqcc2/lib/backend/regalloc.ml`

Skill-perspective check: ran. I loaded `omo:remove-ai-slops` and `omo:programming`, plus the Rust programming reference and code-smells reference. Result: the cleanup resolves the prior high-maintenance issues from those perspectives. No tests were changed, so there are no deletion-only, tautological, path-bridge, or implementation-mirroring tests to reject. The remaining concerns are recorded as risks, not blockers.

## Findings by Severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

1. `src/codegen/regalloc/division_copy.rs:18-32` remains a non-reference peephole, but it is now narrow enough to accept for Task60.
   - Resolution of prior blocker: the broad dividend setup rewrite was removed from `src/codegen/regalloc/rewrite.rs`; the remaining logic is isolated in `division_copy.rs` and called only through `cleanup_redundant_moves` (`src/codegen/regalloc/rewrite.rs:21-25`).
   - Necessity: fresh Chapter 20 default gate passes, and `briggs_dont_coalesce.c` currently hits the harness limit exactly at 7 register-to-register moves. The current assembly loads the division dividend directly into `%eax` before `cdq/idiv`, avoiding the extra pre-dividend register copy that caused the earlier blocker.
   - Narrowness/guarding: the peephole only matches `mov src -> saved_reg; mov saved_reg -> ax; <one instruction not mentioning saved_reg>; cdq/cqo; div/idiv; mov ax -> saved_reg`, with width checks and an explicit saved-register-use guard on the intervening instruction (`division_copy.rs:19-32`, `49-98`, `114-168`).
   - Test adequacy: accepted as covered by the fresh Chapter 20 default/no-coalescing gates plus the targeted division probe recorded below. Residual risk: the helper does not separately reject a future direct division operand that mentions `saved_reg`; current codegen routes divisors through `%r10`, so this is not a current correctness blocker.

### LOW

1. Pre-existing oversized files remain outside the cleanup scope.
   - `src/codegen/codegen.rs`: 2006 -> 1958 pure LOC.
   - `src/driver.rs`: 278 -> 278 pure LOC.
   - These did not newly cross the 250 pure-LOC ceiling in Task60 and should be handled by a separate split/refactor, not this coalescing cleanup.

2. `division_copy.rs` runs through the common cleanup path for both default and `--no-coalescing` modes.
   - This does not collapse the two paths: fresh assembly and harness probes prove they remain distinct.
   - Risk is scope clarity only; if future requirements define `--no-coalescing` as “no late copy cleanup beyond redundant self-moves,” this helper should be gated.

## Prior Blocker Verification

### 1. Dividend peephole scope drift

Status: RESOLVED / ACCEPTED WITH WATCH RISK.

- Old broad logic is gone from `src/codegen/regalloc/rewrite.rs`; current rewrite cleanup delegates to `cleanup_destructive_dividend_copies` and then removes exact redundant moves (`rewrite.rs:21-25`).
- New helper is isolated in `src/codegen/regalloc/division_copy.rs` and under 250 pure LOC.
- Fresh Chapter 20 default gate passed after cleanup.
- Targeted division probe in `/tmp` compiled, linked, and ran correctly in both default and `--no-coalescing` modes.

### 2. Files crossing 250 LOC

Status: RESOLVED FOR NEWLY CROSSED FILES.

Pure LOC command used:

```text
awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|--)/' <file> | wc -l
```

| File | HEAD pure LOC | Current pure LOC | Result |
| --- | ---: | ---: | --- |
| `src/codegen/regalloc/allocate.rs` | 239 | 180 | fixed |
| `src/codegen/regalloc/graph.rs` | 237 | 231 | fixed |
| `src/codegen/fixup.rs` | 235 | 33 | fixed |
| `src/codegen/replace_pseudos.rs` | 474 | 197 | improved below ceiling |
| `src/codegen/regalloc/coalesce.rs` | new | 236 | under ceiling, warning band |
| `src/codegen/regalloc/division_copy.rs` | new | 157 | under ceiling |
| `src/codegen/fixup/split.rs` | new | 238 | under ceiling, warning band |
| `src/codegen/replace_pseudos/split.rs` | new | 199 | under ceiling |
| `src/codegen/replace_pseudos/move_split.rs` | new | 125 | under ceiling |
| `src/codegen/xmm.rs` | new | 11 | under ceiling |

Still oversized but not newly crossed by this cleanup: `src/codegen/codegen.rs` 1958 and `src/driver.rs` 278.

### 3. Duplicated XMM classifier

Status: RESOLVED.

- Shared helper exists at `src/codegen/xmm.rs:1-12`.
- Uses found in `src/codegen/fixup/split.rs`, `src/codegen/replace_pseudos/split.rs`, `src/codegen/regalloc/rewrite.rs`, and `src/codegen/regalloc/coalesce.rs`.
- No duplicate local `is_xmm_binary` helpers remain in changed source.

## Required Scope/Fidelity Checks

- `git diff -- tests`: PASS, empty. `git status --short -- tests` and untracked-test scan were also empty.
- Source/test/harness bridge: PASS. Exact scan found only generic comments (`src/driver.rs:6`, `src/ir/mod.rs:15`), not source-path, chapter/test-name, harness, system-C, or interpreter bridge logic.
- No `source_path_hint`, `SystemAssemblySanitizerOptions`, `compile_with_system_cc_frontend`, `evaluate_program`, `test_compiler`, `latest-only`, `chapter20`, or `chapter_20` production bridge hits.
- Reserved registers: PASS.
  - GP allocatable set excludes R10/R11: `src/codegen/regalloc/types.rs:58-73`, `93-110`.
  - XMM allocatable set excludes XMM14/XMM15: `src/codegen/regalloc/types.rs:74`, `89`, `110`.
  - R10/R11/XMM14/XMM15 usages remain scratch/fixup/codegen uses, not allocator hardregs.
- Default and `--no-coalescing` paths: PASS.
  - Default enables coalescing in `src/driver.rs:54-59`.
  - `--no-coalescing` disables it in `src/driver.rs:84-101`.
  - Allocation branches on `input.options.coalescing_enabled` in `src/codegen/regalloc/allocate.rs:121-149`.
  - Fresh copy-heavy probe shows distinct assembly and fewer allocator-relevant moves in default mode: no-coalescing 1 vs default 0.
- OCaml reference alignment: PASS for coalescing core and allocatable register sets.
  - Briggs/George/coalesce loop reference: `nqcc2/lib/backend/regalloc.ml:385-468`, `584-604`.
  - GP hardregs exclude R10/R11: `nqcc2/lib/backend/regalloc.ml:607-612`.
  - XMM hardregs exclude XMM14/XMM15: `nqcc2/lib/backend/regalloc.ml:614-636`.

## Commands Run

```text
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
cat /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/code-smells.md

git status --short
git diff --name-status
git diff --stat
git diff -- tests
git status --short -- tests
git ls-files --others --exclude-standard tests
git diff --check
git diff -- Cargo.toml Cargo.lock

git diff -- src/codegen/regalloc/allocate.rs src/codegen/regalloc/graph.rs src/codegen/regalloc/rewrite.rs src/codegen/regalloc/mod.rs src/codegen/fixup.rs src/codegen/replace_pseudos.rs src/codegen/codegen.rs src/codegen/mod.rs src/compiler.rs src/driver.rs src/pipeline.rs
nl -ba src/codegen/regalloc/division_copy.rs
nl -ba src/codegen/xmm.rs
nl -ba src/codegen/regalloc/abi_liveness.rs
nl -ba src/codegen/regalloc/graph_pseudos.rs
nl -ba src/codegen/fixup/split.rs
nl -ba src/codegen/replace_pseudos/split.rs
nl -ba src/codegen/replace_pseudos/move_split.rs
nl -ba nqcc2/lib/backend/regalloc.ml | sed -n '380,640p'

rg -n "source_path_hint|SystemAssemblySanitizerOptions|compile_with_system_cc_frontend|evaluate_program|test_compiler|latest-only|test_name|test-name|chapter_20|chapter20|interpreter|system[_ -]?(?:c|cc)|harness" src -S || true
rg -n "is_xmm_binary|AddDouble|SubDouble|MultDouble|SseDivDouble|XorDouble" src/codegen src/codegen/regalloc -S
rg -n "all_hardregs|caller_saved_regs|contains\(|Reg::R10|Reg::R11|XMM\(14\)|XMM\(15\)" src/codegen/regalloc src/codegen/fixup src/codegen/replace_pseudos src/codegen/codegen.rs src/codegen/xmm.rs src/codegen/assembly.rs -S

git show HEAD:<tracked file> | awk ... ; awk ... <current file>   # HEAD/current pure LOC comparison
awk ... <changed/new file> | wc -l                                # current pure LOC table

cargo fmt --all -- --check
cargo test --release
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing
./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only

# copy-heavy distinct-path probe in /tmp
./target/release/rustcc --no-coalescing -S /tmp/.../no/copy-heavy.c
./target/release/rustcc -S /tmp/.../default/copy-heavy.c
gcc /tmp/.../no/copy-heavy.s -o /tmp/.../no/copy-heavy
gcc /tmp/.../default/copy-heavy.s -o /tmp/.../default/copy-heavy
diff -u /tmp/.../no/copy-heavy.s /tmp/.../default/copy-heavy.s

# targeted division legality probe in /tmp
./target/release/rustcc --no-coalescing -S /tmp/.../div_cases_no.c
./target/release/rustcc -S /tmp/.../div_cases_default.c
gcc /tmp/.../div_cases_no.s -o /tmp/.../div_cases_no
gcc /tmp/.../div_cases_default.s -o /tmp/.../div_cases_default
/tmp/.../div_cases_no
/tmp/.../div_cases_default
```

## Fresh Check Results

- `cargo fmt --all -- --check`: PASS, exit 0.
- `cargo test --release`: PASS, 10 tests passed.
- Chapter 20 no-coalescing gate: PASS, 66 tests OK, exit 0.
- Chapter 20 default coalescing gate: PASS, 66 tests OK, exit 0.
- `git diff --check`: PASS, exit 0.
- Copy-heavy probe: PASS, both modes compile/link/run with same return code; default has fewer allocator-relevant register-to-register moves (0 vs 1) and assembly differs as expected.
- Targeted division probe: PASS, both modes compile/link/run with exit 0; default and no-coalescing both preserve division semantics on signed `long` quotient/remainder and self-division cases.
- No W22/full regression run was required or performed.

## Blockers

None.

## Remaining Risks

- `division_copy.rs` is still a late peephole outside the OCaml coalescing algorithm. It is narrow and currently justified by Chapter 20 move-count legality, but future edits should either keep it tightly tied to destructive integer division lowering or gate it if `--no-coalescing` is later defined to disable all late copy cleanup.
- Pre-existing oversized `src/codegen/codegen.rs` and `src/driver.rs` remain architectural cleanup candidates.

## Final Recommendation

APPROVE. The prior blockers are resolved: no tests changed, no bridge logic was introduced, reserved scratch registers remain non-allocatable, the default/no-coalescing paths are distinct and freshly gated, newly crossed LOC violations were split below 250, and the duplicated XMM classifier was replaced by a small shared helper.
