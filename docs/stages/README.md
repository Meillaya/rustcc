# Stage Pseudocode Index

This directory is the pseudocode-only implementation guide for `rustcc`.
It intentionally does **not** contain real Rust implementation. Use these pages as chapter-by-chapter notes before filling in source code yourself.

## How to use these pages

1. Read the chapter page before editing code.
2. Translate the pseudocode into real Rust in the existing `src/` modules.
3. Run the matching commands from `docs/book/test-map.md` and `.omx/plans/full-implementation-pseudocode-plan.md`.
4. Record real verification evidence only after you run the commands yourself.

## Chapter pages

| Chapter | Page | Focus |
|---:|---|---|
| 0 | [`ch00-driver-harness-contract.md`](ch00-driver-harness-contract.md) | Driver, harness, artifact, and phase-envelope contract |
| 1 | [`ch01-minimal-compiler.md`](ch01-minimal-compiler.md) | Minimal return-only compiler |
| 2 | [`ch02-unary-operators.md`](ch02-unary-operators.md) | Unary `-` and `~`, parentheses |
| 3 | [`ch03-binary-operators.md`](ch03-binary-operators.md) | Binary operators and precedence |
| 4 | [`ch04-logical-and-relational-operators.md`](ch04-logical-and-relational-operators.md) | Logical, equality, relational operators |
| 5 | [`ch05-local-variables.md`](ch05-local-variables.md) | Locals, assignments, lvalues |
| 6 | [`ch06-if-and-conditional-expressions.md`](ch06-if-and-conditional-expressions.md) | `if`/`else` and ternary expressions |
| 7 | [`ch07-compound-statements.md`](ch07-compound-statements.md) | Compound statements and scopes |
| 8 | [`ch08-loops.md`](ch08-loops.md) | Loops, `break`, `continue` |
| 9 | [`ch09-functions.md`](ch09-functions.md) | Functions, declarations, calls, ABI basics |
| 10 | [`ch10-globals-and-storage-classes.md`](ch10-globals-and-storage-classes.md) | Globals, linkage, `static`, `extern` |
| 11 | [`ch11-long-integers.md`](ch11-long-integers.md) | `long` and width-aware codegen |
| 12 | [`ch12-unsigned-integers.md`](ch12-unsigned-integers.md) | Unsigned integer conversions and comparisons |
| 13 | [`ch13-floating-point.md`](ch13-floating-point.md) | `double`, SSE/XMM, float conversions |
| 14 | [`ch14-pointers.md`](ch14-pointers.md) | Pointers, address, dereference |
| 15 | [`ch15-arrays-and-pointer-arithmetic.md`](ch15-arrays-and-pointer-arithmetic.md) | Arrays, decay, pointer arithmetic |
| 16 | [`ch16-characters-and-strings.md`](ch16-characters-and-strings.md) | `char`, character and string literals |
| 17 | [`ch17-dynamic-memory-support.md`](ch17-dynamic-memory-support.md) | `sizeof`, `void`, `void*`, dynamic-memory support |
| 18 | [`ch18-structures.md`](ch18-structures.md) | Structs/unions, layout, aggregate ABI |
| 19 | [`ch19-optimizations.md`](ch19-optimizations.md) | TACKY CFG and optimizations |
| 20 | [`ch20-register-allocation.md`](ch20-register-allocation.md) | Liveness, interference, coloring, spills |

## Verification rule

These files are guidance. A chapter is not complete until the real compiler code is written and the relevant official tests pass. Keep actual evidence out of these pages until it has been freshly run.
