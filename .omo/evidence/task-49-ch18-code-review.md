# Task 49 Chapter 18 Union Code Review

Date: 2026-07-08
Workspace: `/home/mei/projects/rustcc`
Verdict: **PASS**

Task 49 completion blocker: **No**. The union-core implementation is scoped and passes the validate-stage gate. Remaining full `--union` failures are W19-T3 ABI/fixture surfaces, not evidence of a task-49 union-core regression.

## Scope reviewed

Changed product files since HEAD:

- `src/ast/decl.rs`
- `src/ast/mod.rs`
- `src/ast/ty.rs`
- `src/parse/parser.rs`
- `src/semantics/resolve.rs`
- `src/semantics/typecheck.rs`
- `src/codegen/type_table.rs`
- `src/ir/lower.rs`

Required evidence read: `.omo/evidence/task-49-ch18-union-implementation.txt`.

## Verification performed

- `git diff --stat HEAD`: 8 source files changed, 321 insertions / 89 deletions.
- `git diff --check HEAD`: PASS, no whitespace errors.
- LSP diagnostics on all modified Rust files: PASS, no diagnostics found.
- `cargo build --release`: PASS.
- `cargo test --release`: PASS, 10 binary unit tests; 0 lib/doc tests.
- `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union --stage validate`: PASS, 286 tests OK.
- Full `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union`: expected red, `failures=31, errors=5`; failure names are confined to `valid/parameters`, `valid/params_and_returns`, and `valid/extra_credit/libraries/{classify_unions,param_passing,union_retvals}` ABI areas.

## Required checks

### No source-content bridge / hard-coded test routing

PASS. No matches in changed source for bridge/test-name patterns:

- `source_has_`
- `should_defer_parse_to_system_frontend`
- `semantic_error_that_should_parse`
- `likely_parse_error`
- `likely_struct_or_union_parse_error`
- `support::source`
- `include_str!`
- `read_to_string`
- hard-coded chapter/test path literals in changed product source

### No test or harness weakening

PASS. `git diff --name-only HEAD` contains only `src/**` product files. No `tests/**`, harness, runner, expected-result, `Cargo.toml`, or lockfile changes are present.

### Unsafe / unwrap / expect / panic additions

PASS with one non-blocking cleanup note. No added `unsafe`, `unwrap()`, `expect()`, or `panic!` calls. One added `unreachable!()` occurs in the new union initializer branch; see LOW-1.

### Kind-conflict / tag-scope behavior

PASS. `src/semantics/resolve.rs:905-976` stores tag entries with `AggregateKind`, rejects same-scope struct/union tag-kind conflicts, and makes nearest tag-name lookup stop even when the nearest declaration has the wrong kind. This preserves one C tag namespace and avoids falling through to an outer different-kind tag.

### Union initializer semantics

PASS. `src/semantics/typecheck.rs:697-705` enforces exactly one union initializer-list element and validates it against the first member. `src/ir/lower.rs:180-203` emits static first-member initialization plus zero padding to union size; `src/ir/lower.rs:1593-1641` handles local union aggregate initialization and same-union copy.

### Union layout: max size/alignment, offset 0

PASS. `src/semantics/typecheck.rs:904-925` assigns every union member offset `0`, tracks maximum member size, tracks maximum member alignment, and rounds final size up to alignment.

### Global type_table reset / leak risk

PASS. `src/semantics/typecheck.rs:32` still calls `type_table::reset()` at the start of typechecking. `src/codegen/type_table.rs:31-33` clears both the complete-entry table and declared-tag set, so union declarations do not persist across compiler invocations in-process.

### W19-T3 ABI overclaim

PASS. The implementation evidence explicitly says full Chapter 18 `--union` remains red because W19-T3 ABI work is outstanding. Current full-run failure names confirm this: parameter passing, aggregate returns, ABI classification, and union ABI library tests. The change should not be claimed as W19-T3 complete.

## Findings by severity

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

#### LOW-1: Added panic-style `unreachable!()` in production lowerer

File: `src/ir/lower.rs:1607-1609`

Issue: The new union initializer branch checks `!matches!(init, Expr::InitializerList(_))` and then destructures with an `else { unreachable!() }`. The invariant is sound in current control flow, but this is still a panic-style escape hatch in production code and was newly added for task 49.

Risk: Low. Normal parser/typechecker flow should not reach it incorrectly, but future refactors could turn a recoverable lowering/type error into a compiler panic.

Minimal fix: Replace the `let ... else { unreachable!() }` with an `if let Expr::InitializerList(items) = init { ... } else { return Err(anyhow::anyhow!("lower: initializer target is not aggregate")); }`, or restructure to bind `items` in the existing `matches!` branch.

Blocks task 49: **No**.

#### LOW-2: Aggregate static-initializer diagnostic still says "array"

File: `src/semantics/typecheck.rs:132-133` and `src/semantics/typecheck.rs:198-199`

Issue: The condition now applies to `Type::Array | Type::Struct | Type::Union`, but the error text remains `static array initializer must be constant`.

Risk: Low. Behavior is correct; only the diagnostic is stale/misleading for structs/unions.

Minimal fix: Change the message to `static aggregate initializer must be constant` or choose kind-specific wording.

Blocks task 49: **No**.

## Recommendation

**PASS**. No blocking code-quality, bridge, test-weakening, union-core semantic, union-layout, tag-scope, or type-table reset issues were found. Treat W19-T3 ABI/library failures as explicitly out of scope for task 49 unless the task definition is widened.
