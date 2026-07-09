# Task 53 / W20-T3 Code Review 2 — UCE after unit-test removal

VERDICT: PASS

Review date: 2026-07-08T21:29:03-04:00
Workspace: `/home/mei/projects/rustcc`
HEAD inspected: `57fe882`
Review scope: current uncommitted Task 53 source diff plus `.omo/evidence/task-53-unreachable-code-code-review.md` and `.omo/evidence/task-53-unreachable-code-fix.txt`.

## Re-review decision

The previous rejection blocker is resolved. `src/ir/unreachable_code_elim.rs` no longer contains a `#[cfg(test)] mod tests` / `#[test]` compiler-phase unit test, and `cargo test --release` now reports 10 tests, not 11. Required UCE harness gate passed. This task can proceed to adversarial gate.

## Required skill-perspective check

- Loaded and consulted `omo:remove-ai-slops` from `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md`.
- Loaded and consulted `omo:programming` from `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md`.
- Loaded and consulted Rust-specific programming reference `/home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md`.
- Skill-perspective result: no current violation. The removed compiler-phase unit test eliminates the prior test-shape/slop blocker. The remaining production UCE implementation is bounded (149 pure LOC), uses existing CFG/TACKY seams, introduces no new dependency, no `unsafe`, and no implementation-mirroring or tautological test remains in the UCE module.

## Artifacts inspected

- Previous review: `.omo/evidence/task-53-unreachable-code-code-review.md`
  - Prior verdict: `REJECT` solely for a compiler-phase Rust unit test in `src/ir/unreachable_code_elim.rs:184-221`.
  - Prior required fix: remove the `#[cfg(test)]` UCE test module and rely on official harness/manual evidence.
- Fix evidence: `.omo/evidence/task-53-unreachable-code-fix.txt`
  - Executor claimed only the test block was removed, production UCE logic unchanged, and gates rerun.
  - Re-review treated this evidence as untrusted and independently verified the claims below.

## Policy/source references re-checked

- `.omo/plans/c-compiler-rust.md:91`: official `tests/test_compiler` Python harness is the compiler-correctness verification path.
- `.omo/plans/c-compiler-rust.md:121`: no Rust unit tests for compiler phases.
- `.omo/plans/c-compiler-rust.md:1850-1863`: Task 53 acceptance is `--chapter 19 --latest-only --eliminate-unreachable-code`.
- `nqcc2/lib/optimizations/unreachable_code_elim.ml:13-104`: reference UCE shape: remove unreachable blocks, useless jumps, useless labels, empty blocks.
- `nqcc2/lib/optimizations/optimize.ml:4-35`: optimization order and fixed-point loop.

## Source/diff findings

- Current source diff remains scoped to UCE wiring:
  - `src/ir/mod.rs:30` registers `mod unreachable_code_elim;`.
  - `src/ir/opt.rs:17` imports UCE; `src/ir/opt.rs:45-50` runs `OptPass::UnreachableCodeElim` while leaving `CopyPropagation | DeadStoreElim` as no-ops.
  - `src/pipeline.rs:85-86` pushes `OptPass::UnreachableCodeElim` only when `optimization_flags.eliminate_unreachable_code` is set.
  - `src/ir/unreachable_code_elim.rs:13-34` preserves program metadata and maps functions through UCE.
  - `src/ir/unreachable_code_elim.rs:62-66` mirrors the reference UCE sequence.
  - `src/ir/unreachable_code_elim.rs:70-92` removes blocks not reachable from `Entry` using `Cfg::reachable_block_ids()`.
  - `src/ir/unreachable_code_elim.rs:96-181` implements useless jump/label/empty-block cleanup.
- Test removal verified:
  - `grep -n '#\[cfg(test)\]\|#\[test\]' src/ir/unreachable_code_elim.rs` produced no output.
  - `grep -RIn '#\[cfg(test)\]\|#\[test\]' src/ir` produced no output.
  - `cargo test --release` output showed `running 10 tests` for `src/main.rs`; no UCE test was listed.
- No copy-prop/DSE/regalloc implementation was introduced by this diff:
  - `src/ir/opt.rs:50` still leaves `OptPass::CopyPropagation | OptPass::DeadStoreElim` as no-ops.
  - source scan hits for regalloc/replace-pseudos were pre-existing comments/modules, not new regalloc implementation.
- No harness weakening found:
  - `git status --short -- Cargo.toml Cargo.lock tests` produced no output.
  - `git diff -- Cargo.toml Cargo.lock tests` produced no output.
- No forbidden bridge fingerprints found in `src`, `tests`, `Cargo.toml`, or `Cargo.lock` for:
  - `evaluate_program`, `evaluate_with_system_cc`, `compile_with_system_cc_frontend`, `SystemAssemblySanitizerOptions`, `system_c_to_assembly`, `system_c_syntax_check`, `source_has_`, `SystemCc`, `bridge`.
- No `unsafe`, `unwrap(`, or `expect(` found in changed source files.
- Pure LOC counts:
  - `src/ir/unreachable_code_elim.rs`: 149
  - `src/ir/opt.rs`: 33
  - `src/pipeline.rs`: 89
  - `src/ir/mod.rs`: 11

## Exact commands inspected/run

```text
printf 'WORKING: task-53-review-2 - loading skill criteria\n'
sed -n '1,240p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
sed -n '241,520p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/remove-ai-slops/SKILL.md
sed -n '1,190p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
sed -n '191,378p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/SKILL.md
sed -n '1,260p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md
sed -n '261,520p' /home/mei/.codex/plugins/cache/sisyphuslabs/omo/4.16.0/skills/programming/references/rust/README.md

pwd
git status --short
sed -n '1,260p' .omo/evidence/task-53-unreachable-code-code-review.md
sed -n '1,260p' .omo/evidence/task-53-unreachable-code-fix.txt

sed -n '20,32p;88,106p;116,124p;1848,1865p' .omo/plans/c-compiler-rust.md
git diff -- src/ir/mod.rs src/ir/opt.rs src/pipeline.rs
git diff --no-index -- /dev/null src/ir/unreachable_code_elim.rs || true
nl -ba src/ir/unreachable_code_elim.rs
nl -ba src/ir/opt.rs
nl -ba src/pipeline.rs | sed -n '1,130p'
nl -ba src/ir/mod.rs

grep -n '#\[cfg(test)\]\|#\[test\]' src/ir/unreachable_code_elim.rs || true
grep -RIn '#\[cfg(test)\]\|#\[test\]' src/ir || true
git status --short -- Cargo.toml Cargo.lock tests
git diff -- Cargo.toml Cargo.lock tests
rg -n 'CopyPropagation|DeadStoreElim|replace_pseudos|regalloc|Register|allocate|propagate|dead_store|copy_prop|eliminate_unreachable_code|UnreachableCodeElim' src/ir/opt.rs src/pipeline.rs src/ir/unreachable_code_elim.rs src/ir/mod.rs
rg -n 'evaluate_program|evaluate_with_system_cc|compile_with_system_cc_frontend|SystemAssemblySanitizerOptions|system_c_to_assembly|system_c_syntax_check|source_has_|SystemCc|system cc|bridge' src tests Cargo.toml Cargo.lock || true
rg -n '\bunsafe\b|unwrap\(|expect\(' src/ir/unreachable_code_elim.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs || true
for f in src/ir/unreachable_code_elim.rs src/ir/opt.rs src/pipeline.rs src/ir/mod.rs; do printf '%s ' "$f"; awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#|\/\*!|\*)/' "$f" | wc -l; done

nl -ba nqcc2/lib/optimizations/unreachable_code_elim.ml | sed -n '1,140p'
nl -ba nqcc2/lib/optimizations/optimize.ml | sed -n '1,90p'
nl -ba src/ir/cfg/types.rs | sed -n '1,180p'
nl -ba src/ir/cfg/build.rs | sed -n '1,220p'
nl -ba src/driver.rs | sed -n '60,115p'
nl -ba src/compiler.rs | sed -n '180,250p'

cargo fmt --all -- --check
cargo check --release
git diff --check
cargo test --release
cargo build --release
./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code

git rev-parse --short HEAD
date -Iseconds
grep -n 'only verification path\|No Rust unit tests for compiler phases\|53\. W20-T3\|--eliminate-unreachable-code' .omo/plans/c-compiler-rust.md
grep -n 'fn eliminate_unreachable_code_program\|fn optimize\|fn eliminate_unreachable_blocks\|fn eliminate_useless_jumps\|fn eliminate_useless_labels\|fn remove_empty_blocks' src/ir/unreachable_code_elim.rs
grep -n 'UnreachableCodeElim\|CopyPropagation | OptPass::DeadStoreElim\|eliminate_unreachable_code' src/ir/opt.rs src/pipeline.rs src/ir/mod.rs
```

## Required gate results

```text
cargo fmt --all -- --check
# exit 0, no output

cargo check --release
# exit 0
# Finished `release` profile [optimized] target(s) in 0.05s

git diff --check
# exit 0, no output

cargo test --release
# exit 0
# src/lib.rs: running 0 tests, ok
# src/main.rs: running 10 tests; 10 passed; 0 failed
# Doc-tests rustcc: running 0 tests, ok

cargo build --release
# exit 0
# Finished `release` profile [optimized] target(s) in 0.01s

./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code
# exit 0
# Ran 15 tests in 0.336s, OK
```

## Findings by severity

### CRITICAL

- None.

### HIGH

- None.

### MEDIUM

- None.

### LOW

1. **Scope hygiene remains noisy outside the Task 53 source diff.**
   - Evidence: `git status --short` still shows modified `.omo/boulder.json` and many older untracked `.omo/evidence/task-*` files unrelated to Task 53.
   - Impact: not a code correctness or gate blocker, but the eventual handoff/commit should keep Task 53 scoped to intentional source/evidence artifacts.

2. **UCE still silently returns unchanged output on CFG construction failure.**
   - Evidence: `src/ir/unreachable_code_elim.rs:42-49` returns `changed: false` if `cfg::tacky_function_cfg(&function)` fails.
   - Impact: unchanged from review 1 and not shown to affect valid accepted programs or the required gate. Consider fail-fast behavior when optimizer error plumbing exists.

## Blockers

- None. The prior blocker (`#[cfg(test)]` / `#[test]` compiler-phase UCE unit test) is resolved.

## Final recommendation

Proceed to adversarial gate.

codeQualityStatus: CLEAR
recommendation: APPROVE
reportPath: `.omo/evidence/task-53-unreachable-code-code-review-2.md`
blockers: []
