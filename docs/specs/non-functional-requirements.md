# Non-Functional Requirements

## Correctness

- The compiler SHALL preserve observable behavior for accepted programs.
- Optimizations SHALL be semantics-preserving.
- ABI-sensitive backend behavior SHALL be treated as correctness, not polish.

## Determinism

- Repeated builds of the same input SHOULD produce stable outputs and diagnostics.
- Stage boundaries SHOULD be deterministic and debuggable.

## Maintainability

- Compiler phases SHOULD remain separated behind explicit module boundaries.
- New chapter features SHOULD map cleanly to tests, docs, and requirements.

## Portability

- The compiler SHOULD remain compatible with the environments assumed by the bundled test suite.
- Platform-specific codegen differences SHOULD stay in backend/toolchain boundaries.

## Testability

- Every major feature SHOULD be testable at the latest-only chapter level.
- Intermediate stages SHOULD be inspectable independently.
