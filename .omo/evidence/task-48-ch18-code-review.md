# Task 48 Chapter 18 Core Structs Code Review

Verdict: PASS
Date: 2026-07-08
Workspace: /home/mei/projects/rustcc

## Scope Reviewed

Focused read-only review of current task 48 changes, including `git diff` plus untracked `src/codegen/type_table.rs`:

- `src/ast/decl.rs`, `src/ast/expr.rs`, `src/ast/item.rs`, `src/ast/mod.rs`, `src/ast/ty.rs`
- `src/codegen/type_table.rs`, `src/codegen/codegen.rs`, `src/codegen/replace_pseudos.rs`
- `src/parse/parser.rs`
- `src/semantics/resolve.rs`, `src/semantics/typecheck.rs`, `src/semantics/label_loops.rs`
- `src/ir/lower.rs`, `src/ir/tacky.rs`

Required evidence read:

- `.omo/evidence/task-48-ch18-structs-implementation.txt`
- `.omo/evidence/task-48-adversarial-verify.txt`

## Verification Evidence Used

From the current task evidence:

- `cargo build --release`: PASS.
- `cargo test --release`: PASS, 10 unit tests.
- LSP diagnostics on modified Rust files: PASS/no diagnostics reported.
- Core Chapter 18 no-struct-ABI gate: PASS, `Ran 161 tests, OK`.
- Full Chapter 18 latest-only: expected red only in W19-T3 ABI-owned `valid/parameters` and `valid/params_and_returns` directories.
- Chapter 17 latest-only: PASS, `Ran 70 tests, OK`.
- Chapter 16 latest-only: PASS, `Ran 72 tests, OK`.
- Manual probes covered acceptance member access, nested layout/offset/sizeof, invalid member rejection, stale state, and malformed struct input.

Additional review command run now:

- `cargo check`: PASS.
- `lsp_diagnostics` was run for all modified/source-focus files listed above; all returned `diagnosticCount: 0`.

## Severity Summary

- CRITICAL: 0
- HIGH: 0
- MEDIUM: 0
- LOW: 1

## Findings

### LOW

1. `src/ir/lower.rs:1501` — New product-code `unreachable!()` in struct initializer lowering.
   - Issue: The new `let Expr::InitializerList(items) = init else { unreachable!() };` is guarded by an immediately preceding non-initializer return, so it is not a current correctness blocker. However, it is still a panic-like macro added in production lowering code and is unnecessary.
   - Risk: If this function is later refactored and the guard drifts, malformed/internal input could panic instead of returning a normal compiler error.
   - Minimal fix: Replace the `unreachable!()` arm with a normal `return Err(anyhow::anyhow!("lower: initializer target is not aggregate"));` or restructure the branch as a single `match init`.
   - Blocks task 48 completion: No.

## Required Checks

### Source-content bridge

PASS. Scan of changed implementation files found no runtime bridge to OCaml/nqcc2/source-content/Python and no `Command::new`/`std::process` additions. Hits are existing mirror-reference comments and normal project comments only.

### Test/harness modifications and test weakening

PASS. `git diff --name-status -- tests test .github`, cached diff, and `git status --short -- tests test .github` produced no tracked test/harness changes. No deletion-only tests, harness filters, weakened assertions, or test-name conditionals were introduced in product source.

### Hard-coded test-name behavior

PASS. Source scan found no product-code branches keyed on chapter-18 test paths/names such as `valid/`, `parameters`, `params_and_returns`, `no_structure_parameters`, `latest-only`, or specific fixture names. Test-directory strings appear only in evidence artifacts, not in `src/` behavior.

### Unsafe / unwrap / expect / panic additions

PASS with LOW note above. No added `unsafe`, `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `println!`, or `eprintln!` lines were found in the task diff/untracked source. One added `unreachable!()` exists at `src/ir/lower.rs:1501`; it is covered as a LOW non-blocking finding.

### Broad catch-all silently dropping IR

PASS. No new catch-all branch was found that silently drops executable IR. `StructDecl` no-op arms in lowering are expected because struct declarations populate the type table during typechecking and do not emit runtime instructions.

### Tag-scope correctness obvious flaws

PASS. The resolver adds a separate `TagScopes` namespace, resolves member types through that namespace, and pushes/pops tag scopes for function bodies, blocks, and `for` scopes. Current verification includes core tag/scope tests plus a stale-state manual probe. I did not find an obvious tag leakage or ordinary-identifier namespace collision in the reviewed paths.

### Type table global reset / leak risk

PASS for task 48; risk noted. `src/codegen/type_table.rs` uses process-global `OnceLock<Mutex<...>>` tables and `typecheck()` resets them at the start of a compile. The current single-compile pipeline and stale-state probe show no observed leak across compiler invocations. Future concurrent in-process compilation would be unsafe because one compile could reset another compile's type table; this is a non-blocking architecture risk for the current task split.

### ByteArray / struct copy correctness

PASS. Struct object storage is represented as `OperandType::ByteArray`, `Instruction::CopyBytes` copies qwords then trailing bytes through registers, and pseudo replacement handles memory destinations needed by member/aggregate stores. Current manual/core tests cover nested struct copy, padding/offset, static and automatic initialization, and member stores/loads. I found no W19-T1-blocking byte-copy defect.

### W19-T1 scope fidelity / no union or ABI overclaim

PASS. The implementation is scoped to core structs: declarations, member layout/access, initializers, value copy, and no-ABI/no-parameter core behavior. I found no union implementation and no struct ABI parameter/return implementation claim. Full Chapter 18 failures remain confined to W19-T3 ABI directories per the adversarial evidence.

### Comments / TODO / debug leftovers

PASS. No added `TODO`, `FIXME`, debug print, `dbg!`, or commented-out code was found. Existing comments are mirror/reference comments consistent with the project style.

## Oversized-module Risk Notes

Non-blocking inherited risk: several touched modules are already far above the 250 pure-LOC review threshold:

- `src/ir/lower.rs`: 2333 pure LOC; task diff adds the largest new surface here.
- `src/codegen/codegen.rs`: 1582 pure LOC.
- `src/parse/parser.rs`: 1190 pure LOC.
- `src/semantics/typecheck.rs`: 848 pure LOC.
- `src/semantics/resolve.rs`: 776 pure LOC.
- `src/codegen/replace_pseudos.rs`: 453 pure LOC.
- `src/semantics/label_loops.rs`: 287 pure LOC.

Per task instruction, I am not blocking on broad inherited refactors. The concrete risk is reviewability: future Chapter 18 ABI work should avoid further growing `lower.rs`/`codegen.rs` without a focused split plan if correctness issues appear.

## Completion Gate Assessment

No CRITICAL, HIGH, or MEDIUM issue was found. The single LOW finding does not block task 48 completion. The prior adversarial rejection was artifact absence; this review artifact satisfies the missing code-review/slop gate from the evidence trail.

Final verdict: PASS
