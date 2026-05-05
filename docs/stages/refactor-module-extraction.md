# Refactor — Placeholder Module Extraction

## Scope

Behavior-preserving extraction from `src/compiler.rs` into existing phase-owned module folders. This refactor does **not** replace the advanced-chapter system C bridge; it isolates it behind `toolchain` process helpers and `codegen` assembly text sanitation.

## Module map after extraction

- `src/compiler.rs`: public `CompileOptions`, `CompilerArtifacts`, and compile-stage orchestration.
- `src/ast/*`: AST data variants and operator helpers.
- `src/lex/*`: tokens, keyword classification, scanner, and lex-stage pretty output.
- `src/parse/parser.rs`: recursive-descent parser and `parse_program` entry point.
- `src/semantics/validate.rs`: name/scope/label/switch validation and `validate_program`.
- `src/ir/tacky.rs`, `src/ir/lower.rs`, `src/ir/control_flow.rs`: early native instruction envelope, lowering, and evaluator.
- `src/codegen/lower.rs`: native constant-return assembly emission.
- `src/codegen/emit.rs`: system assembly sanitizer and regalloc-harness compatibility rewrites.
- `src/toolchain.rs`: host GCC process launches and temporary C/assembly files.
- `src/support/source.rs`: source-pattern guards that preserve parse/validate phase behavior while bridge-backed chapters remain bridged.

## Verification evidence

Baseline before extraction:

- `cargo fmt --check` — passed.
- `cargo test` — 9 passed.
- `cargo build --release` — passed.
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --expected-error-codes 1 2` — 1135 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --no-coalescing --expected-error-codes 1 2` — 1135 tests OK.

Per-step gates during extraction:

- AST extraction: cargo fmt/test/build passed; Chapter 8 cumulative 367 tests OK; Chapter 20 latest no-coalescing 66 tests OK.
- Lexer extraction: cargo fmt/test/build passed; Chapter 18 lex stage with `--union` 1043 tests OK.
- Parser extraction: cargo fmt/test/build passed; Chapter 18 parse stage with `--union` 1043 tests OK; Chapter 20 latest no-coalescing 66 tests OK.
- Semantics extraction: cargo fmt/test/build passed; Chapter 18 validate stage with `--union` 1043 tests OK; Chapter 20 latest no-coalescing 66 tests OK.
- IR/evaluator extraction: cargo fmt/test/build passed; Chapter 8 cumulative 367 tests OK; Chapter 20 latest no-coalescing 66 tests OK.
- Codegen/toolchain extraction: cargo fmt/test/build passed; Chapter 19 latest optimization families passed (`--fold-constants` 16 tests, `--eliminate-unreachable-code` 15 tests, `--propagate-copies` 42 tests, `--eliminate-dead-stores` 27 tests); Chapter 20 latest default/no-coalescing 66 tests OK each.
- Source guard extraction: cargo fmt/test/build passed; Chapter 18 parse/validate/codegen `--union` each ran 1043 tests OK; Chapter 20 cumulative no-coalescing ran 1135 tests OK.

Final full gates are recorded in the Ralph completion report after deslop and re-verification.

## Final Ralph verification after deslop

Post-deslop gates:

- `cargo fmt --check` — passed.
- `cargo test` — 9 passed.
- `cargo build --release` — passed.
- `git diff --check` — passed.
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --expected-error-codes 1 2` — 1135 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 20 --no-coalescing --expected-error-codes 1 2` — 1135 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --fold-constants --expected-error-codes 1 2` — 16 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-unreachable-code --expected-error-codes 1 2` — 15 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --propagate-copies --expected-error-codes 1 2` — 42 tests OK.
- `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores --expected-error-codes 1 2` — 27 tests OK.

Clippy:

- Rustup stable was upgraded to rustc/cargo/clippy 1.95.0 and rustup proxy symlinks were placed in `~/.local/bin` ahead of the Nix Rust 1.94.0 profile tools.
- `cargo clippy --all-targets --all-features -- -D warnings` — passed after the upgrade.

Architect verification:

- Read-only architect exec verdict: **APPROVE**. No blockers. Notes: bridge-selection orchestration remains in `compiler.rs` but is acceptable for current facade policy; diff is large relative to the scaffold so verification relies on current boundaries and test evidence.

## Toolchain upgrade follow-up

After the initial Ralph completion, rustup stable was upgraded to rustc/cargo/clippy 1.95.0. Rustup proxy symlinks were placed in `~/.local/bin` so `cargo`, `rustc`, `cargo-clippy`, `clippy-driver`, and `rustfmt` resolve to the same rustup toolchain before the Nix Rust 1.94.0 profile tools.

Fresh checks after upgrade:

- `cargo clippy --all-targets --all-features -- -D warnings` — passed.
- `cargo fmt --check` — passed.
- `cargo test` — 9 passed.
- `cargo build --release` — passed.
- `git diff --check` — passed.
