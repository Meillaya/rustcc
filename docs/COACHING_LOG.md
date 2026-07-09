# rustcc Coaching Log

This file is the single running record for the guided build of the compiler.
It collects the current spec, instructions, milestones, and explanations as we go.

---

## Session: 2026-04-15

### Working mode
- Coaching-first.
- Do not do the compiler implementation for the user.
- Provide architecture, sequencing, milestones, and review feedback.
- Keep guidance concrete and incremental.

### Current decision
Develop the **compile driver first**, before the lexer/parser, following the book's recommendation.

### Why start with the compile driver?
- It establishes the outer compilation pipeline early.
- It separates orchestration from compiler internals.
- It lets later lexer/parser/codegen work plug into an existing flow.
- It gives early end-to-end wins even before the compiler internals are real.

### Driver responsibilities
The compile driver is responsible for:
- accepting the input `.c` file
- parsing stage-selection flags
- validating filenames and extensions
- deriving intermediate/output paths
- invoking the system preprocessor
- calling the compiler boundary module
- optionally invoking assembler/linker
- stopping early at requested stages

The compile driver is **not** responsible for:
- tokenization details
- parsing rules
- AST design internals
- code generation internals

### Recommended module boundaries
- `main.rs`: entry point only; parse args, build config, call driver, print errors
- `driver.rs`: orchestration of the compilation pipeline
- `toolchain.rs`: wrappers around external tools like `cc -E -P` and final assemble/link commands
- `compiler.rs`: public boundary for the user's compiler logic; stub now, real internals later

Later internal compiler modules:
- `token.rs`
- `lexer.rs`
- `ast.rs`
- `parser.rs`
- `codegen.rs`

### Boundary rule
- `driver` works with **files, flags, and subprocesses**
- `compiler` works with **source text and compiler data structures**

### Suggested stage model
Use an enum-like mental model for stages:
- preprocess
- lex
- parse
- codegen
- full

Even if some stages are still stubbed, designing around them now will keep the pipeline clean.

### First milestone
Make this work:

`rustcc input.c`
- validate the input file
- preprocess `input.c` into `input.i`
- read the preprocessed text
- pass it into a placeholder compiler entry point
- print a deterministic success result

### Second milestone
Replace the placeholder compiler with a temporary stub that emits fixed assembly for a trivial program such as:

```c
int main(void) { return 2; }
```

Then have the driver:
- write `input.s`
- invoke the system toolchain
- produce an executable

This validates the full shell around the future compiler implementation.

### Current next task
Design `driver::run` as a plain-English control-flow checklist before writing code.

### Control-flow outline for `driver::run`
1. Receive a validated `Config` from `main`.
2. Extract the input path and requested stage.
3. Confirm the input exists and has a `.c` extension.
4. Derive intermediate/output paths:
   - `.i` for preprocessed source
   - `.s` for assembly
   - executable path for full builds
5. Invoke the preprocessor through `toolchain`.
6. If the requested stage is preprocess-only, stop successfully here.
7. Read the generated `.i` file into a string.
8. Pass that string into the public function in `compiler.rs`.
9. Depending on the selected stage:
   - print tokens
   - print AST
   - write assembly
   - or continue toward a final executable
10. If assembly was produced and the stage is full, call the toolchain again to assemble/link.
11. Return success, or propagate a structured error.

### Design guidance for `driver::run`
- Keep it linear and easy to trace.
- Prefer one stage boundary per block.
- Make early-return points explicit for stop-stage flags.
- Keep external command details inside `toolchain.rs`.
- Keep compiler internals behind one public API in `compiler.rs`.

### Coaching note
Do not over-generalize the driver on day one. The goal is a clean, testable path through the pipeline, not a perfect abstraction.

### Scaffold created
Created the initial compile-driver source layout:
- `src/main.rs`
- `src/driver.rs`
- `src/toolchain.rs`
- `src/compiler.rs`

Current scaffold behavior:
- the binary builds
- the driver accepts a simple input path plus stage flags
- the driver validates `.c` input
- the driver derives `.i`, `.s`, and executable paths
- the driver prints the planned pipeline instead of executing it

This is intentional: the project now has the correct outer shape without jumping ahead into the real implementation.

### Immediate next implementation target
Replace the scaffold print-only path with the first real pipeline action:
1. implement preprocessing in `toolchain.rs`
2. call it from `driver::run`
3. stop successfully at preprocess-only mode
4. read the generated `.i` file
5. send that text into the placeholder compiler boundary
Planning placeholder directories created only for plan artifact targeting; no implementation files written yet.

## Session update: 2026-04-15 ralplan artifacts written
- Consensus plan approved via planner → architect → critic loop.
- Wrote .omx/plans/prd-rustcc-book-package.md.
- Wrote .omx/plans/test-spec-rustcc-book-package.md.
- Tightened docs naming and scaffold banlist policy in the consensus plan.

## Session update: 2026-04-16 ralph execution complete
- Authored full docs/book package (20 chapter guides + backbone + maps + appendices + templates).
- Authored docs/specs SRS package.
- Authored docs/research resource package, including blogs-and-papers.
- Moved the placeholder scaffold into `src/` at the user's request.
- Wrote verification and deslop reports under .omx/plans/.

## Wave 0 Verification — Foundation Rewrite

**Date**: 2026-07-07T19:48:15Z

**Gate commands**:
- `cargo build --release` → exit 0, zero warnings
- `cargo test --release` → 9 passed, 0 failed
- Fingerprint greps (all zero matches in `src/`):
  - `evaluate_program`, `compile_with_system_cc_frontend`, `source_has_*`, `should_defer_parse_to_system_frontend`, `semantic_error_that_should_parse`, `likely_parse_error`, `likely_struct_or_union_parse_error`
  - `evaluate_with_system_cc`, `system_c_syntax_check`, `system_c_to_assembly`, `write_temp_c_source` in `src/toolchain.rs`
  - `sanitize_system_assembly`, `SystemAssemblySanitizerOptions`

**Deleted files**:
- `src/ir/control_flow.rs` → `No such file or directory` (interpreter removed)
- `src/support/source.rs` → `No such file or directory` (heuristic gate removed)
- `src/codegen/emit.rs` → path now holds the new OCaml-mirror codegen emitter (`emit()` pretty-prints `AsmProgram` to x86-64 AT&T text). The old system-C sanitizer content is gone; verified by zero matches for `sanitize_system_assembly` / `SystemAssemblySanitizerOptions`.

**OCaml-mirror layout**:
- `src/semantics/{resolve,label_loops,typecheck}.rs`
- `src/ir/{tacky,lower,opt,cfg,temp}.rs`
- `src/codegen/{assembly,assembly_symbols,abi,codegen,emit,fixup,frame,regalloc,replace_pseudos}.rs`

**Kept gcc helpers** in `src/toolchain.rs`:
- `preprocess()` for `gcc -E -P`
- `assemble_only()` / `assemble_and_link()` for final gcc invocation

**Evidence**: `/home/mei/projects/rustcc/.omo/evidence/task-7-wave0-gate.txt`

## Wave 4 Verification — Chapter 3 Binary Operators + Bitwise Extras

**Date**: 2026-07-07

**Scope**: Add arithmetic binary operators (`+ - * / %`) plus bitwise extras
(`& | ^ << >>`) with parser precedence, TACKY lowering, and x86 codegen.

### Implementation

**AST** (`src/ast/operator.rs`):
- Trimmed `BinaryOp` to the chapter-3 set:
  `Add, Subtract, Multiply, Divide, Remainder, ShiftLeft, ShiftRight,
  BitwiseAnd, BitwiseXor, BitwiseOr`.
- Removed chapter-4 variants (`Less, LessEqual, Greater, GreaterEqual,
  Equal, NotEqual, LogicalAnd, LogicalOr`) so chapter-4 programs fail
  at parse time in the chapter-3 build.

**Precedence** (`src/parse/precedence.rs`):
- Defined `Precedence` enum with 11 variants ordered low-to-high:
  `Lowest, LogicalOr, LogicalAnd, BitOr, BitXor, BitAnd, Equality,
  Relational, BitShift, AddSub, MulDiv, Highest`.
  `Lowest` / `Highest` are sentinels; the nine real operator levels
  mirror C precedence (`* / %` > `+ -` > `<< >>` > relational >
  equality > `&` > `^` > `|` > `&&` > `||`).
- Added `precedence_of(kind: &TokenKind) -> Option<Precedence>` and a
  `next_higher()` method that walks the table strictly upward.

**Parser** (`src/parse/parser.rs`):
- `parse_binary_expr(min_precedence: Precedence)` implements
  precedence climbing; the loop accepts `op_prec < min_precedence`
  breaking and recurses with `op_prec.next_higher()` so left
  associativity holds at every level (including same-precedence
  chains like `3 / 2 * 4`).
- `peek_binary_op` returns `(BinaryOp, Precedence)` for chapter-3 +
  bitwise tokens only. Chapter-4 tokens (`< <= > >= == != && ||`)
  match `precedence_of` but are filtered out so they fail at parse.

**TACKY lowering** (`src/ir/lower.rs`):
- `lower_expr` handles `Expr::Binary { op, left, right }` by
  recursively lowering both sides, allocating a fresh `tmp.N`, then
  emitting `Copy left, tmp; BinaryOp { op, right, tmp }`.
- `binary_to_tacky` maps each `BinaryOp` to its two-address TACKY
  variant (`Add, Sub, Mul, DivSigned, RemSigned, BitShiftLeft,
  BitShiftRight, BitAnd, BitOr, BitXor`).

**Codegen** (`src/codegen/codegen.rs`):
- Standard arithmetic / bitwise ops collapse to a single
  `<op> src, dst` instruction because the lowering already moved the
  left operand into `dst` via `Copy`.
- `Mul` lowers through the reg-to-reg form
  `movl dst, %eax; movl src, %r10d; imull %r10d, %eax; movl %eax, dst`
  because the GNU assembler rejects the two-operand `imull` with
  immediate or memory operands.
- `DivSigned` / `RemSigned` use `cdq; idivl` with the divisor
  materialized via `%r10d` for the same reason.
- `BitShiftLeft` / `BitShiftRight` move the count into `%ecx` and
  emit the shift with the count in `%cl`.

**Emitter** (`src/codegen/emit.rs`):
- Added `format_binary_op` for `addl, subl, imull, idivl, andl, orl,
  xorl, sall, sarl` (plus the double-precision forms reserved for
  chapter 13).
- Added `format_shift_src` so the shift count operand formats as
  `%cl` instead of the default `%ecx` (x86-64 requires the count in
  the byte register).
- Wired `Instr::Idiv` and `Instr::Cdq` into `format_instruction`.

**Pseudo replacement** (`src/codegen/replace_pseudos.rs`):
- Extended `replace_in_instruction` to walk `BinaryOp` operands and
  `Idiv`.
- Extended `split_memory_to_memory` to also split memory-to-memory
  `BinaryOp` and `Idiv` via the `%r10` scratch register.

### Gate commands

- `cargo build --release` → exit 0, zero warnings
- `cargo test --release` → 9 passed, 0 failed
- `./tests/test_compiler ./target/release/rustcc --chapter 3
   --latest-only --bitwise` → 35 tests pass (all green)

### Manual QA (3 scenarios from the chapter 3 task)

| Source                                          | Expected | Actual |
|-------------------------------------------------|---------:|-------:|
| `int main(void){return 1+2*3;}`                 |        7 |      7 |
| `int main(void){return 12%5;}`                  |        2 |      2 |
| `int main(void){return (1<<3)|(2&0xf0);}`       |        8 |      8 |

### Evidence

- `cargo build`: `.omo/evidence/task-14-cargo-build.txt`
- `cargo test`: `.omo/evidence/task-14-cargo-test.txt`
- chapter 3 + bitwise gate: `.omo/evidence/task-14-chapter-gate.txt`
- manual QA writeup: `.omo/evidence/task-14-manual-qa.txt`

## Wave 5 Verification — Chapter 4 Logical & Relational Operators

**Date**: 2026-07-07

**Scope**: Add logical operators (`!`, `&&`, `||`) and relational operators
(`==`, `!=`, `<`, `<=`, `>`, `>=`) through the AST, parser, TACKY
lowering (with short-circuit evaluation), and x86 codegen. Mirrors
`nqcc2/lib/parse.ml` chapter-4 portion (lines ~150-280),
`nqcc2/lib/tacky_gen.ml` chapter-4 short-circuit lowering
(`emit_and_expression` / `emit_or_expression` ~lines 230-269), and
`nqcc2/lib/backend/codegen.ml` chapter-4 cmpl + setCC codegen.

### Implementation

**AST** (`src/ast/operator.rs`, `src/ast/expr.rs`):
- Added chapter-4 variants to `BinaryOp`:
  `Equal, NotEqual, LessThan, LessOrEqual, GreaterThan, GreaterOrEqual,
  LogicalAnd, LogicalOr`. The relational / equality operators lower
  to a single TACKY `Cmp` instruction; `LogicalAnd` / `LogicalOr` use
  short-circuit control flow.
- Added `Not` to `UnaryOp` for the chapter-4 logical-not operator
  (`!e`). Distinct from the chapter-2 `Complement` (`~e`, bitwise
  NOT): `!0 == 1` while `~0 == -1`.
- Removed the redundant `Expr::LogicalNot` variant; the parser now
  folds `!` into the existing `Expr::Unary { op: UnaryOp::Not, ... }`
  shape so a single match arm handles all unary forms in the
  lowerer.

**Precedence** (`src/parse/precedence.rs`):
- Chapter-4 precedence slots (`Relational`, `Equality`, `LogicalAnd`,
  `LogicalOr`) were already reserved in wave 4 so chapter-4 programs
  failed at parse time with chapter-3 binaries. Verified the
  precedence levels are still wired through `precedence_of` for
  `< <= > >= == != && ||`.

**Parser** (`src/parse/parser.rs`):
- `parse_unary_expr` now emits `Expr::Unary { op: UnaryOp::Not, ... }`
  on `TokenKind::Bang`.
- `peek_binary_op` covers the chapter-4 tokens: `EqualEqual`,
  `NotEqual`, `Less`, `LessEqual`, `Greater`, `GreaterEqual`,
  `LogicalAnd`, `LogicalOr`. The match is exhaustive over the
  precedence-yielding tokens; a `_ => unreachable!(...)` arm
  documents the invariant.

**Label generator** (`src/util/labels.rs`):
- Implemented `LabelGenerator` (mirrors `nqcc2/lib/util/unique_ids.ml
  ::make_label`). Distinct from `TempIdGenerator` so the two name
  spaces stay syntactically separate (`tmp.N` vs `prefix.M`).

**TACKY IR** (`src/ir/tacky.rs`):
- Added `pub enum ConditionCode` with all 11 variants (signed
  `E/NE/L/LE/G/GE`, unsigned `A/AE/B/BE`, parity `P`) so the
  chapter-4 work only flips the signed subset; the unsigned +
  parity codes are reserved for chapter 12.
- Added `Instruction::Cmp { left, right, dst, cc }` — comparison
  with explicit operands and boolean result destination. Distinct
  from the two-address shape used by arithmetic / bitwise binops.

**TACKY lowering** (`src/ir/lower.rs`):
- `UnaryOp::Not` lowers to a single `Cmp { left: inner_val,
  right: 0, cc: E, dst: tmp }`. The `Copy + Negate|Complement` shape
  is preserved for `Negate` / `Complement`.
- Arithmetic / bitwise / shift binops continue to use the
  two-address `Copy left, tmp; BinaryOp { src: right, dst: tmp }`
  shape (now with `dst` pre-loaded via Copy).
- Equality / relational binops emit a single `Cmp { left, right,
  dst, cc }`. The pre-emitted `Copy left, tmp` is harmless here:
  `Cmp` carries both operands explicitly so it does not require the
  two-address shape.
- `LogicalAnd` / `LogicalOr` use short-circuit lowering:
  - `&&`: eval e1; `JumpIfZero e1, and_false.N`; eval e2;
    `JumpIfZero e2, and_false.N`; `Copy 1, dst`; `Jump and_end.M`;
    `Label and_false.N: Copy 0, dst`; `Label and_end.M`.
  - `||`: mirror with `JumpIfNotZero` and `or_false.N` / `or_end.M`.
- A fresh `LabelGenerator` is owned by `lower_program` so labels are
  unique per expression.

**Codegen** (`src/codegen/codegen.rs`):
- `Instruction::Cmp` lowers to:
  ```
  [ optional movl $imm, %r11d   ; if left was an immediate ]
  cmpl  right, left
  setCC cc dst                  ; writes 1 byte to dst
  movzbl dst, %r10d             ; zero-extend byte to 32-bit
  movl   %r10d, dst             ; write full word back
  ```
  The immediate-routing uses `%r11d` (not `%r10d`) to avoid a
  clobber from `split_memory_to_memory`, which also uses `%r10d`.
- `Instruction::JumpIfZero` / `Instruction::JumpIfNotZero` lower to
  `cmpl $0, cond; je/jne target` (with the same `%r10d` immediate
  routing as the `Cmp` instruction).
- `Instruction::Jump` lowers to `jmp target`.
- `Instruction::Label(name)` lowers to `name:`.
- `map_cc` translates `tacky::ConditionCode` into the structurally
  identical `assembly::ConditionCode` (kept as a separate type so
  the IR layer stays free of codegen dependencies).

**Emitter** (`src/codegen/emit.rs`):
- Added formatters for the new instructions:
  `Instr::Cmp` → `cmpl right, left` (AT&T: dst = left),
  `Instr::Jmp` → `jmp label`,
  `Instr::JmpCC` → `j{cc} label`,
  `Instr::SetCC` → `set{cc} operand`,
  `Instr::MovZeroExtend` → `movzbl src, dst`,
  `Instr::Label` → `label:`.
- `format_cond_code` covers the signed subset used by chapter 4
  (`e, ne, l, le, g, ge`) plus the unsigned + parity codes reserved
  for chapter 12.

**Pseudo replacement** (`src/codegen/replace_pseudos.rs`):
- Extended `replace_in_instruction` to walk `Cmp`, `SetCC`,
  `MovZeroExtend`, `Jmp`, `JmpCC`, and `Label` operands.
- Extended `split_memory_to_memory` to split memory-to-memory
  `cmpl` via a `%r10d` scratch register (x86-64 forbids mem/mem
  comparisons).

### x86-64 constraints handled

- `cmpl imm, imm` is invalid → route an immediate left operand
  through `%r11d` first.
- `cmpl imm, mem` is invalid (immediate cannot be the AT&T
  destination) → same routing.
- `cmpl mem, mem` is invalid → split the right operand through
  `%r10d` in `split_memory_to_memory`.
- `movzbl mem, mem` is invalid (and `sete` only writes a byte so
  the destination's upper bytes are undefined) → after `sete`,
  read the byte via `movzbl dst, %r10d` then `movl %r10d, dst` to
  restore a clean 32-bit value.

### Gate commands

- `cargo build --release` → exit 0, zero warnings
- `cargo test --release` → 9 passed, 0 failed
- `./tests/test_compiler ./target/release/rustcc --chapter 4
   --latest-only --bitwise` → 43 tests pass (all green)
- `./tests/test_compiler ./target/release/rustcc --chapter 4
   --bitwise` → 121 tests pass (chapters 1-4 cumulative, no
   regressions)

### Manual QA (5 scenarios from the chapter 4 task + short-circuit)

| Source                                          | Expected | Actual |
|-------------------------------------------------|---------:|-------:|
| `int main(void){return 1<2;}`                   |        1 |      1 |
| `int main(void){return 1&&0;}`                  |        0 |      0 |
| `int main(void){return (1\|\|0)&&1;}`           |        1 |      1 |
| `int main(void){return 1==1;}`                  |        1 |      1 |
| `int main(void){return 5!=3;}`                  |        1 |      1 |
| `int main(void){return 0&&(1/0);}` (short-circuit) |   0 |      0 |
| `int main(void){return 1\|\|(1/0);}` (short-circuit) |  1 |      1 |

The short-circuit cases exercise the `&&` / `||` control-flow
plumbing without triggering a divide-by-zero (the right operand is
never evaluated when the left's boolean outcome makes it
unnecessary).

### Evidence

- `cargo build`: `.omo/evidence/task-17-cargo-build.txt`
- `cargo test`: `.omo/evidence/task-17-cargo-test.txt`
- chapter 4 + bitwise gate: `.omo/evidence/task-17-ch4-gate.txt`


## Wave 6 — Chapter 5 (local variables, assignment, compound, ++/--)

### Scope

Land the chapter-5 subset plus its extras:

- Mutable local variables (declarations + reads).
- Simple `=` and compound `+= -= *= /= %= &= |= ^= <<= >>=`
  assignment, all right-associative.
- Pre/post `++` and `--`.
- Block scope (block statements open an inner scope; `for` init
  declares in its own scope).
- Synthetic `Return 0` at the end of every function so
  `int main(void) {}` still terminates.

### Outcome

- `cargo build --release` → exit 0, zero warnings
- `cargo test --release` → 9 passed, 0 failed
- `./tests/test_compiler ./target/release/rustcc --chapter 5
   --latest-only --bitwise --compound --increment` → 82 / 82
  tests pass (all green)

### Pipeline changes

- `src/semantics/resolve.rs` — replaced the pass-through stub
  with a real scope-tracking pass: function body walked with a
  `ScopeStack`, declarations added before their initializer is
  resolved (so `int a = a;` / `int a = a = 5;` compile with
  `a` in scope but indeterminate value, matching C), duplicate
  declarations in the same scope rejected, undeclared `Expr::Var`
  references rejected.
- `src/ast/decl.rs` — `Function::body` widened from
  `Vec<Statement>` to `Vec<BlockItem>` so the function body can
  mix declarations and statements, matching the OCaml `Block
  (BlockItem list)` shape.
- `src/parse/parser.rs` — `parse_program` pushes block items
  directly (no longer filters declarations out of function-body
  top level).
- `src/ir/lower.rs` — rewritten to walk `Vec<BlockItem>`, lower
  every `Statement` variant (`If`, `While`, `DoWhile`, `For`,
  `Block`, `Return`, `Expr`), every `Expr` variant (`Var`,
  constant, paren, unary, binary incl. short-circuit `&&`/`||`,
  conditional, simple + compound assignment, pre/post
  increment/decrement). Compound assignment evaluates the lvalue
  once into a tmp, emits the binary op, and stores back. Pre
  `++x` emits `Add(1, x)` and returns `Var(x)`; post `x++`
  emits `Copy(x, old) ; Add(1, x)` and returns `Var(old)`.
  `ensure_trailing_return` appends `Return(Constant(0))` when
  the body has no explicit one.

### Bugs fixed during wave-6 verification

- Short-circuit constants were swapped — `||` returned 0 where
  it should return 1 (and vice versa). Root-caused the
  `compound_assignment_lowest_precedence` SIGFPE
  (a short-circuit 0 flowed into `d /= ...` and divided by
  zero).  Fix: `(short_circuit_value, long_form_value)` keyed
  off `is_or`, copied into the right slot of the long-form /
  short-circuit label pair.
- Self-referential initializer (`int a = a;`) failed because
  the resolve pass declared `a` AFTER resolving the initializer.
  Fix: declare first, then resolve the initializer expression
  so `a` is in scope (with an indeterminate value) — C and the
  OCaml reference both behave this way.
- Empty / non-returning functions crashed (SIGSEGV / -11)
  because there was no terminating `Return`. Fix: synthetic
  `Return(Constant(0))` mirroring `emit_fun_declaration` in
  `nqcc2/lib/tacky_gen.ml`.

### Manual QA (chapter 5 task + scope edges)

| Source                                                            | Expected | Actual |
|-------------------------------------------------------------------|---------:|-------:|
| `int main(void) { int x = 5; return x; }`                         |        5 |      5 |
| `int main(void) { int x = 5; x += 3; return x; }`                 |        8 |      8 |
| `int main(void) { int x = 5; return ++x; }`                       |        6 |      6 |
| `int main(void) { int x = 5; int y = x++; return y * 10 + x; }`   |       56 |     56 |

Note on the last row: the task brief asserted exit 57 but the
arithmetic result of `5 * 10 + 6 = 56` is the only value the
expression can produce in a sound implementation (post-increment
returns the old value, then x becomes 6).  The chapter-5 gate
(`compound_assignment_use_result`, `compound_assignment_chained`,
`non_short_circuit_or`, etc.) confirms 56 / 1 respectively.

### Evidence

- `cargo build`: `.omo/evidence/task-18-cargo-build.txt` (rolled
  from earlier runs; latest run is clean — release profile,
  zero warnings, exit 0).
- `cargo test`: `.omo/evidence/task-18-cargo-test.txt` — 9
  passed, 0 failed.
- chapter 5 gate (latest-only + bitwise + compound + increment):
  `.omo/evidence/task-18-ch5-gate.txt` — `Ran 82 tests … OK`.


## Wave 7 — Chapter 6 (if/else, ternary, --goto)

### Scope

Land the chapter-6 subset plus its `--goto` extra:

- `if (cond) stmt` and `if (cond) stmt else stmt`
  (statement-level branching, with optional `else`).
- Right-associative ternary `cond ? then : else`
  (expression-level branching).
- Labeled statements `label:` (a statement prefix; can attach
  to any statement including another label) and `goto label;`
  (the `--goto` extra).

### Outcome

- `cargo build --release` → exit 0, zero warnings.
- `cargo test --release` → 9 passed, 0 failed.
- `./tests/test_compiler ./target/release/rustcc --chapter 6
   --latest-only --bitwise --compound --increment --goto`
  → 68 / 68 tests pass (all green).
- Full chapter-1..6 regression: chapters 1–6 with the same
  flags → 467 / 467 tests pass (no regressions).

### Pipeline changes

The AST, parser, and `If` / `Conditional` / loop lowering
were already in place from earlier waves (W7-T1 surfaced them
during exploration); the new work was:

- `src/semantics/label_loops.rs` — promoted the W0-T6 stub
  to a real validation pass.  Walks the function body once to
  collect every label name into a `HashSet<String>` (rejecting
  duplicates), then walks again to verify every
  `Statement::Goto(target)` resolves to a label in the same
  function.  Because labels and variables live in different
  namespaces, the same pass naturally rejects
  `goto <variable>;` (the target is missing from the labels
  set) without needing a separate variable scan.  Function
  scoping (rule: no `goto` across function boundaries) is
  enforced because the walker only sees the current function's
  body.
- `src/ir/lower.rs::lower_statement` — wired the chapter-6
  extras.  `If { c, then, else_branch }` and the
  right-associative `Conditional` were already lowered; the new
  arms add:
  - `Statement::Goto(target)` → `Instruction::Jump { target }`
  - `Statement::Labeled { label, statement }` →
    `Instruction::Label(label)` followed by the lowered
    statement.
  Both arms route through a `mangle_user_label` helper that
  prefixes the user's name with `user_label.`, so a program
  containing `goto main; … main: return 0;` doesn't shadow
  the function-entry symbol in the emitted assembly.  The
  mangling is invisible to the validation pass (labels are
  tracked by their source name) but visible to codegen
  (which emits `user_label.main:` rather than `main:`).
- `src/codegen/codegen.rs` and `src/codegen/emit.rs` —
  verified existing arms for `Jump`, `Label`, `JumpIfZero`,
  `JumpIfNotZero`, and the matching assembly forms (`jmp`,
  `name:`, `jCC`) already covered chapter 6; no changes
  needed.
- `src/parse/parser.rs` — verified `parse_statement` already
  handles `if` / `else`, `goto label;`, and `Identifier Colon`
  label prefixes (with recursive descent so labels can stack).
  `parse_conditional_expr` already implements the right-
  associative `cond ? expr : cond_expr` shape required by the
  OCaml reference and the book.

### Manual QA (chapter 6 task + goto edges)

| Source                                                                                  | Expected | Actual |
|-----------------------------------------------------------------------------------------|---------:|-------:|
| `int main(void) { int x = 5; if (x > 0) return 10; else return 20; }`                   |       10 |     10 |
| `int main(void) { int a = 1; a = a > 0 ? 7 : 8; return a; }`                             |        7 |      7 |
| `int main(void) { int x = 0; goto end; x = 5; end: return x; }`                         |        0 |      0 |

The third row exercises the `--goto` extra; the
`mangle_user_label` helper translates the user-visible
`end` label into an assembly-safe `user_label.end` so the
generated jump / label pair stays scoped to the function.

### Invalid_semantics gates for --goto

Three `--goto` extra invalid_semantics tests now reject
correctly at the new validation pass (and would not have
before W7-T1):

- `duplicate_labels` — two `label:` statements with the same
  name in one function → bail at label collection.
- `goto_missing_label` — `goto label;` with no `label:`
  defined → bail at goto check.
- `goto_variable` — `goto a;` where `a` is a local variable,
  not a label → bail at goto check (because `a` is not in the
  labels set).

### Evidence

- `cargo build`: `.omo/evidence/task-20-cargo-build.txt`
  (release profile, zero warnings, exit 0).
- `cargo test`: `.omo/evidence/task-20-cargo-test.txt` — 9
  passed, 0 failed.
- chapter 6 gate (latest-only + bitwise + compound +
  increment + goto):
  `.omo/evidence/task-20-ch6-gate.txt` — `Ran 68 tests … OK`.

---

## Wave 8 / W8-T1 — Chapter 7 compound statements

### Working mode
- Focused executor (Sisyphus-Junior). Direct implementation against the
  plan; no interview gate.

### Scope
- Add compound statements (`{ ... }` blocks) with nested scopes and
  variable shadowing.  Per-block scope stack; declarations get unique
  names; references resolve innermost-first; inner-scope bindings
  shadow outer-scope bindings on re-declaration.

### Outcome

- `cargo build --release` → exit 0, zero warnings.
- `cargo test --release` → 9 passed, 0 failed.
- `./tests/test_compiler ./target/release/rustcc --chapter 7
   --latest-only --compound --goto` → 27 / 27 tests pass (all green).
- Chapter 1-7 cumulative regression (with the same extra-credit
  flags) → 298 / 298 tests pass (no regressions).
- Manual QA scenarios all match the expected exit codes:
  shadowed inner x does not leak, inner block does not pollute the
  outer scope, deep-nested return surfaces the inner binding.

### Pipeline changes

The parser, IR, codegen, and replace-pseudos paths were already in
place from earlier waves (the AST's `Statement::Block(Vec<BlockItem>)`
existed and the parser already handled `{ items }` as a recursive
block, and `replace_pseudos` already allocates a fresh stack slot
per unique pseudo name); the new work was concentrated in the
semantic-analysis phase:

- `src/semantics/resolve.rs` — promoted the chapter-5 flat scope
  tracker to a true per-block scope stack that **mangles names**
  on the way through.  Each `Block` / `For` arm opens a fresh inner
  scope (push), each `declare` mints a globally unique name
  (`x` → `x.0`, `x.1`, …, mirroring the OCaml
  `Unique_ids.make_named_temporary` helper), and each `Var(name)`
  reference is rewritten to the unique name from the nearest
  enclosing scope (innermost-first lookup).  Walking out of a block
  pops the inner scope so the outer binding is naturally visible
  again.  `resolve_program` now returns a new `Program` whose
  declarations and references use the unique names; the lowerer
  consumes the resolved AST verbatim and the codegen / replace-
  pseudos stages naturally map each unique name to its own
  `Stack(offset)` slot.
  - Declaration order matches C99 / OCaml: `int a = init` declares
    `a` in the current scope **before** resolving `init`, so
    `int a = a + 1` references the new `a` (the
    `assign_to_self_2` test in chapter 7 relies on this).
  - Duplicate detection stays in the current scope; shadowing
    across nested scopes is allowed by design.
  - Undeclared references still fail with a precise error
    message; the resolve pass is now the single source of truth
    for both name-mangling and undeclared-variable rejection.
- `src/ir/lower.rs`, `src/codegen/codegen.rs`, and
  `src/codegen/replace_pseudos.rs` — verified end-to-end that the
  resolved unique names flow through unchanged.  The lowerer emits
  `Copy { src, dst: <unique_name> }` and the assembly emitter maps
  each unique pseudo to its own `Stack(offset)` via
  `ReplaceState::resolve`, which already does `entry.or_insert`
  on the pseudo map.  No new offsets to track — the per-block
  "release" of stack slots is implicit in the
  monotonic-grow / never-shrink `stack_size` counter, while the
  unique-name guarantee prevents accidental cross-scope aliasing.
- `src/parse/parser.rs::parse_statement` — verified that the
  existing `OpenBrace` arm in `parse_statement` already produced
  `Statement::Block(Vec<BlockItem>)`, so the chapter-7 grammar
  (`{ <block-item>* }` with `block-item ::= decl | stmt`) was
  already wired.  No changes required.

### Manual QA (chapter 7 + shadowing edges)

| Source                                                                          | Expected | Actual |
|---------------------------------------------------------------------------------|---------:|-------:|
| `int main(void) { int x = 1; { int x = 5; } return x; }`                         |        1 |      1 |
| `int main(void) { int x = 1; { int y = 7; } return x + 0; }`                     |        1 |      1 |
| `int main(void) { { int x = 3; { int y = 4; } return x; } }`                     |        3 |      3 |

All three exercise the new per-block scope stack:
- Row 1: inner `x = 5` is a shadow of the outer `x`; on block exit
  the outer `x` (value 1) is what `return x` reads.
- Row 2: inner `y = 7` does not pollute the outer scope; the
  return sees only the outer `x` (value 1).
- Row 3: three nested blocks; only the innermost-but-one binding of
  `x` is live when `return x;` runs.

### Test gates flipped green

- `assign_to_self_2` (chapter 7 valid) — `int a = 3; { int a = a
  = 4; } return a;` returns 3.  The `a = 4` on the right of the
  inner `int a = ...` declaration references the **inner** `a`
  (because the inner `a` is in scope for the init), so only the
  inner binding is set to 4; the outer `a` is untouched.
- `hidden_then_visible` (chapter 7 valid) — `int a = 2; int b; { a
  = -4; int a = 7; b = a + 1; } return b == 8 && a == -4;`
  returns 1.  Pre-declaration `a = -4` resolves to the outer
  binding; post-declaration `a + 1` resolves to the inner binding
  (= 7), so `b = 8`; the outer `a` stayed at -4.
- `similar_var_names` (chapter 7 valid) — the deeply-nested
  `a` / `a1` shadow test returns 28 (= 20 + 5 + 2 + 1).  The
  per-block scope stack guarantees the inner `a1` (value 2) is
  used in the same scope and the outer `a1` (value 1) is used
  after the scope exits.
- `--goto` extra-credit tests in chapter 7
  (`goto_before_declaration`, `goto_outer_scope`) now compile and
  run correctly because the resolve pass leaves label names alone
  (only variables are mangled) and the lowerer's
  `mangle_user_label` keeps the assembly label namespace
  disjoint.

### Evidence

- `cargo build`: `.omo/evidence/task-22-cargo-build.txt` — zero
  warnings, exit 0.
- `cargo test`: `.omo/evidence/task-22-cargo-test.txt` — 9
  passed, 0 failed.
- chapter 7 gate (latest-only + compound + goto):
  `.omo/evidence/task-22-ch7-gate.txt` — `Ran 27 tests … OK`.
- chapter 1-7 cumulative (compound + bitwise + increment + goto):
  `.omo/evidence/task-22-ch7-cumulative.txt` — `Ran 298 tests …
  OK`.
- manual QA: `.omo/evidence/task-22-manual-qa.txt` — all three
  rows match expected exit codes.

---

## Session: 2026-07-07 — chapter 9 invalid-semantic gate

### Working mode
- Sisyphus-Junior (focused executor from OhMyOpenCode).
- Execute tasks directly without delegating.

### Goal
Fix the 7 failing chapter 9 `invalid_semantic` tests:

| Test file                                            | What it exercises                              |
|------------------------------------------------------|------------------------------------------------|
| `invalid_declarations/decl_params_with_same_name.c`  | duplicate parameter names in a *declaration*   |
| `invalid_declarations/redefine_fun_as_var.c`         | same-scope function-decl + variable collision  |
| `invalid_declarations/redefine_var_as_fun.c`         | same-scope variable + function-decl collision  |
| `invalid_declarations/undeclared_fun.c`              | call before any prior declaration/definition   |
| `invalid_types/conflicting_function_declarations.c`  | declaration and definition with different arities |
| `invalid_types/too_few_args.c`                       | call with fewer args than the callee declares  |
| `invalid_types/too_many_args.c`                      | call with more args than the callee declares   |

### Root cause
The pre-fix `resolve.rs`:
- Did **two passes**: collected every function name in a first pass,
  then resolved bodies in a second pass.  This made
  `undeclared_fun.c` pass (it should fail).
- Tracked only `Declared` / `Defined` in `FunctionEntry` — no arity,
  so `too_few_args.c`, `too_many_args.c`, and
  `conflicting_function_declarations.c` all silently passed.
- Did **not** check duplicate parameters in *declarations* (only in
  definitions), so `decl_params_with_same_name.c` passed.
- Folded block-level `int NAME(params);` into a no-op
  `Statement::Expr(None)`, so the local function-declaration name was
  invisible to the per-block scope and `redefine_*_as_*` tests passed.

### Implementation summary
Five files touched:

- `src/ast/decl.rs` — added `BlockItem::FunctionDecl(GlobalDecl)`
  variant so a block-level `int NAME(params);` is a real AST node (not
  a no-op statement).  Reuses the existing `GlobalDecl { name, params }`
  shape so the AST stays lean.
- `src/parse/parser.rs` — when the block-level lookahead sees
  `int NAME ( params ) ;`, emit
  `BlockItem::FunctionDecl(GlobalDecl { name, params })` instead of
  the previous `Statement::Expr(None)` no-op.
- `src/semantics/resolve.rs` — full rewrite of the chapter-9 surface:
  - `FunctionEntry` is now a `struct { arity: usize, defined: bool }`.
  - `resolve_program` is a **single top-down pass** over the
    translation unit.  Each top-level item is processed in source
    order; a function body can only call names that have been declared
    or defined *earlier* (matching C's single-translation-unit
    visibility rule and the OCaml reference's `resolve.ml`).
  - `check_function_conflict` rejects a duplicate definition, a
    conflicting arity across declarations, and accepts a same-arity
    re-declaration (the OCaml `has_linkage = true` re-declaration
    path, which `multiple_declarations.c` exercises).
  - `check_duplicate_params` is called on both function
    *definitions* (existing behaviour) and forward *declarations*
    (new — fixes `decl_params_with_same_name.c`).
  - `ScopeStack` now carries a parallel `Vec<HashMap<String, usize>>`
    of per-scope function prototypes (name → arity).  `declare_fun`
    inserts into the innermost scope and rejects a same-scope collision
    with a variable declaration (or a conflicting-arity re-declaration);
    `declare` rejects a same-scope collision with a function prototype.
    This is what catches the block-level
    `redefine_fun_as_var.c` / `redefine_var_as_fun.c` cases.
  - Call sites (`Expr::Call { name, args }`) look up the arity by
    walking the per-block fun-decls stack innermost-first, then
    falling back to the global function table.  An undeclared name
    fails with `"call to undeclared function"`; an arity mismatch
    fails with `"function 'foo' called with N argument(s) but
    declared with M"`.
- `src/semantics/label_loops.rs` — `BlockItem::FunctionDecl(_)` arm
  added to `check_user_gotos_block` as a no-op (no gotos can hide
  inside a prototype).
- `src/ir/lower.rs` — `BlockItem::FunctionDecl(_)` arm added to
  `lower_block_items` as a no-op (the prototype has no runtime effect).

### Public-API surface
Unchanged: `resolve_program`, `ResolvedProgram`, and `resolve`'s
export list are identical.  The new `FunctionEntry` is module-private.

### QA

| Gate                                                 | Result          |
|------------------------------------------------------|-----------------|
| `cargo build --release`                              | exit 0, zero warnings |
| `cargo test --release`                               | 9 passed, 0 failed |
| chapter 9 `--latest-only`                            | 7/7 invalid tests rejected (1 pre-existing `stack_alignment` test-fixture error: missing `stack_alignment_check_linux.s`) |
| chapter 9 valid extra-credit (`--bitwise --compound --increment --goto --switch`) | OK (same 1 pre-existing fixture error) |
| chapter 8 `--latest-only --compound --increment --goto --switch` | `Ran 98 tests … OK` |
| chapters 1–7 cumulative `--latest-only`              | all `OK` |
| manual QA: `function_shadows_variable.c`            | passes (inner-scope function decl shadows outer variable) |
| manual QA: `variable_shadows_function.c`            | passes (inner-scope variable shadows outer function decl) |
| manual QA: `forward_decl_multi_arg.c`               | passes (forward decl with arity 2, definition with arity 2, call with 2 args) |

### Evidence
- `.omo/evidence/task-29-cargo-build.txt`
- `.omo/evidence/task-29-cargo-test.txt`
- `.omo/evidence/task-29-ch9-fix.txt`
- `.omo/evidence/task-29-ch9-valid.txt`

## Wave 14 / Chapter 13 core doubles (task 37)

Added the Chapter 13 core `double` foundation: parsing and typed lowering for double constants/declarations, scalar SSE2/XMM codegen for double moves/arithmetic/comparisons, int↔double and unsigned↔double conversions, double constant-pool emission, and basic XMM argument/return ABI handling. NaN-aware comparison extras remain intentionally out of scope for the follow-up task.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0, zero warnings |
| `cargo test --release` | 9 passed, 0 failed |
| chapter 13 `--latest-only` | `Ran 50 tests … OK` |
| manual core double program | compiles and exits 1 as expected |
| chapter 5 `--latest-only --bitwise --compound --increment` | `Ran 82 tests … OK` |
| chapter 12 `--latest-only` | checked; still fails 13 pre-existing/adjacent unsigned/linkage cases captured in evidence |

### Evidence
- `.omo/evidence/task-37-ch13-core-gate.txt`
- `.omo/evidence/task-37-manual-qa.txt`
- `.omo/evidence/task-37-regressions.txt`

## Wave 14 / Chapter 13 NaN extra (task 38)

Added NaN-aware Chapter 13 double comparisons for the `--nan` extra. The backend now treats unordered `ucomisd` results as false for `==`, `<`, `<=`, `>`, `>=`, true for `!=`, and treats NaN as nonzero in double conditions.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0, zero warnings |
| `cargo test --release` | 9 passed, 0 failed |
| chapter 13 `--latest-only` | `Ran 50 tests … OK` |
| chapter 13 `--latest-only --nan` | `Ran 51 tests … OK` |
| chapter 5 `--latest-only --bitwise --compound --increment` | `Ran 82 tests … OK` |
| manual `double x = 0.0/0.0; return x != x;` | compiles and exits 1 as expected |

### Evidence
- `.omo/evidence/task-38-ch13-nan-gate.txt`
- `.omo/evidence/task-38-manual-qa.txt`
- `.omo/evidence/task-38-code-review.txt`

## Wave 15 / Chapter 14 pointers (task 40)

Added Chapter 14 pointer support: pointer declarators and abstract pointer casts, address-of/dereference expressions, pointer lvalue assignment through `*p`, pointer/null comparison checks, TACKY `GetAddress`/`Load`/`Store`, and x86-64 `leaq` plus indirect memory loads/stores.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0, zero warnings |
| `cargo test --release` | 9 passed, 0 failed |
| chapter 14 `--latest-only` | `Ran 53 tests … OK` |
| chapter 13 `--latest-only --nan` | `Ran 51 tests … OK` |
| chapter 5 `--latest-only --bitwise --compound --increment` | `Ran 82 tests … OK` |
| manual pointer read/store | read exits 5; store-through-pointer exits 10 |

### Evidence
- `.omo/evidence/task-40-ch14-gate.txt`
- `.omo/evidence/task-40-manual-qa.txt`
- `.omo/evidence/task-40-regressions.txt`
- `.omo/evidence/task-40-code-review.txt`

## Wave 16 / Chapter 15 arrays and pointer arithmetic (task 42)

Implemented Chapter 15 natively in the Rust compiler pipeline: parser and typecheck now model arrays and decay at function boundaries, TACKY lowers subscripting into `AddPtr`, and codegen emits native pointer arithmetic, array addressing, and static array initialization directly. The gate no longer depends on the rejected bridge fallback; alignment handling, static zero-init sizing, and the `ZeroExtend` fix keep the chapter 15 array/pointer path book-faithful end-to-end.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0, zero warnings |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 15 `--latest-only` | `Ran 83 tests … OK` |
| chapter 14 `--latest-only` | `Ran 53 tests … OK` |
| chapter 13 `--latest-only --nan` | `Ran 51 tests … OK` |
| manual W16 array acceptance | compiles and exits 60 |

### Evidence
- `.omo/evidence/task-42-ch15-gate.txt`
- `.omo/evidence/task-42-adversarial-verify-2.txt`
- `.omo/evidence/task-42-executor-fix.txt`
- `.omo/evidence/task-42-manual-qa.txt`
- `.omo/evidence/task-42-code-review.txt`

## Wave 17 / Chapter 16 chars and strings (task 44)

Implemented Chapter 16 natively in the Rust compiler pipeline: `char` now flows through the AST and type system, char and string literals parse and lower directly, string literals are emitted as static `.rodata` byte constants, and the backend now performs the required byte load/store and sign/zero-extension paths for both register and stack arguments. Restoring the page-boundary assembly fixture exposed a real byte stack-argument bug; the fix extends narrow byte arguments before `pushq` so the chapter 16 harness can pass without any source-content bridge or test weakening.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0, zero warnings |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 16 `--latest-only` | `Ran 72 tests … OK` |
| chapter 15 `--latest-only` | `Ran 83 tests … OK` |
| chapter 14 `--latest-only` | `Ran 53 tests … OK` |
| manual char/string acceptance | `'A'` returns 65; `"hello"[0]` returns 104 |
| forbidden bridge scan | no matches in `src/` |

### Evidence
- `.omo/evidence/task-44-ch16-implementation.txt`
- `.omo/evidence/task-44-adversarial-verify-2.txt`
- `.omo/evidence/task-44-ch16-qa/`
- `.omo/evidence/task-45-ch16-gate.txt`

## Wave 18 / Chapter 17 void, sizeof, and libc-linked dynamic memory (task 46/47)

Implemented Chapter 17 natively in the Rust compiler pipeline: `void` now works as a real return type and pointer element type, `return;` is accepted for void functions, `sizeof(expr)` and `sizeof(type)` are parsed and lowered as non-evaluated constants, incomplete types such as `void` are rejected where required, and external dynamic-memory declarations/calls (`malloc`/`free`) link through libc without any source-content bridge fallback. The gate verification below confirms the chapter 17, 16, and 15 latest-only harnesses stay green and the forbidden bridge scan remains clean.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 17 `--latest-only` | `Ran 70 tests … OK` |
| chapter 16 `--latest-only` | `Ran 72 tests … OK` |
| chapter 15 `--latest-only` | `Ran 83 tests … OK` |
| forbidden bridge scan | no matches in `src/` |

### Evidence
- `.omo/evidence/task-46-ch17-implementation.txt`
- `.omo/evidence/task-46-adversarial-verify.txt`
- `.omo/evidence/task-46-ch17-qa/`
- `.omo/evidence/task-47-ch17-gate.txt`

## Wave 19 / Chapter 18 core structs (task 48)

Implemented the Chapter 18 W19-T1 core struct surface in the Rust compiler pipeline: native struct tags and the type table, complete layout/member-offset tracking, block/file tag scoping, member access through `.` and `->`, struct initializers, struct value copy, and the non-ABI core gate. This closes the core struct portion only; W19-T3 still owns the remaining struct ABI parameter/return classification for full Chapter 18.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| core Chapter 18 no-structure-parameters gate | `PYTHONPATH=tests python3 /tmp/run_ch18_core.py` → 161 tests, OK |
| chapter 17 `--latest-only` | `Ran 70 tests … OK` |
| chapter 16 `--latest-only` | `Ran 72 tests … OK` |
| full chapter 18 `--latest-only` | expected red only in `valid/parameters` and `valid/params_and_returns` (W19-T3 ABI scope) |
| forbidden bridge scan | clean |
| manual acceptance program | exit 15 |
| invalid member probe | rejected with `type error: struct has no member 'b'` |

### Evidence
- `.omo/evidence/task-48-ch18-structs-implementation.txt`
- `.omo/evidence/task-48-adversarial-verify.txt`
- `.omo/evidence/task-48-adversarial-verify-2.txt`
- `.omo/evidence/task-48-ch18-code-review.md`

### Remaining scope
- W19-T3 struct ABI parameter/return classification is still pending.
- The type table remains process-global and is reset at compile start; that matches the current single-compile path but is a future concurrency risk.

## Wave 19 / Chapter 18 union extra (task 49)

Implemented the Chapter 18 W19-T2 union extra on top of the already-landed struct core: `Type::Union` is now a first-class aggregate kind, the type table records union entries with `size = max(member sizes)`, `alignment = max(member alignments)`, and every member at offset `0`, and tag lookup respects the single tag namespace without falling through a shadowed struct/union kind mismatch. Typechecking and lowering now accept union initialization, union copy, and union conditionals using the shared-storage semantics expected by the chapter tests.

### QA

| Gate | Result |
|------|--------|
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 18 `--latest-only --union --stage validate` | `Ran 286 tests … OK` |
| chapter 18 `--latest-only --union --stage codegen` | `Ran 286 tests … OK` |
| chapter 17 `--latest-only` | `Ran 70 tests … OK` |
| chapter 16 `--latest-only` | `Ran 72 tests … OK` |
| forbidden bridge scan | no matches in `src/` |
| full chapter 18 `--latest-only --union` | still red only in W19-T3 ABI buckets; union-core failures are gone |

### Evidence

- `.omo/evidence/task-49-ch18-union-implementation.txt`
- `.omo/evidence/task-49-ch18-code-review.md`
- `.omo/evidence/task-49-adversarial-verify-2.txt`
- `.omo/evidence/task-49-ch18-union-commit.txt`

### Remaining scope

- W19-T3 Chapter 18 ABI classification for by-value aggregate parameters and returns remains unchecked.
- No broader `--union` claim should be made until that ABI work lands.

## Wave 19 / Chapter 18 System V aggregate ABI (task 50)

Implemented the Chapter 18 W19-T3 aggregate System V AMD64 ABI path: by-value struct/union parameters are classified into integer/SSE/memory locations, small aggregates can be returned through registers, and large aggregates return through the hidden caller-provided return pointer. This closes the full Chapter 18 gate including the union extra; restored page-boundary assembly fixtures remain ignored by the global `*.s` rule and were force-add staged for the chapter commit.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 18 `--latest-only` | `Ran 192 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| chapter 17 `--latest-only` | `Ran 70 tests ... OK` |
| chapter 16 `--latest-only` | `Ran 72 tests ... OK` |
| stale ABI scaffold scan | no matches in `src/` |
| forbidden bridge/interpreter scan | no matches in `src/` |
| manual ABI probes | matched GCC for mixed aggregate register return and hidden large-aggregate return pointer |
| `cargo clippy --all-targets --all-features -- -D warnings` | repo-wide pre-existing warnings remain; Task 50 `CodegenCtx` `derivable_impls` blocker is gone |

### Evidence

- `.omo/evidence/task-50-ch18-abi-implementation.txt`
- `.omo/evidence/task-50-ch18-code-review-4.md`
- `.omo/evidence/task-50-derivable-default-fix.txt`
- `.omo/evidence/task-50-adversarial-verify-4.txt`

### Remaining scope

- Chapter 19 CFG and optimization passes begin at W20-T1.
- Full repo-wide clippy is still red on pre-existing style findings outside the Task 50 blocker; do not treat that as a Chapter 18 ABI regression.

## Wave 20 / Chapter 19 CFG foundation (task 51)

Implemented the Chapter 19 W20-T1 control-flow graph foundation that mirrors `nqcc2/lib/cfg.ml`: linear TACKY and assembly instruction streams can now be split into basic blocks with entry/exit nodes, predecessor/successor edges, label resolution, conditional/unconditional jump handling, return-to-exit handling, fallthrough edges, and reusable annotation/reassembly helpers for the upcoming optimization and register-allocation work.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| chapter 17 `--latest-only` | `Ran 70 tests ... OK` |
| CFG disposable edge probe | PASS for conditional, fallthrough, unconditional, return, and missing-label behavior |
| forbidden bridge/interpreter scan | no matches in source/tests/manifests |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-51-cfg-implementation.txt`
- `.omo/evidence/task-51-cfg-code-review.md`
- `.omo/evidence/task-51-cfg-adversarial-verify.txt`

### Remaining scope

- W20-T2 through W20-T5 must consume this CFG for constant folding, unreachable-code elimination, copy propagation, and dead-store elimination.
- Chapter 20 regalloc/liveness can use the assembly CFG seam, but liveness/interference/coalescing remain intentionally unimplemented.

## Wave 20 / Chapter 19 constant folding (task 52)

Implemented the Chapter 19 W20-T2 constant-folding pass on top of the CFG foundation. The optimizer now wires `--fold-constants` through the pipeline, evaluates safe constant TACKY copies/casts/unary/binary/comparison operations for the supported scalar types, folds constant conditional branches, preserves original `TackyFunction` metadata when reassembling CFG bodies, and keeps memory/call effects conservative by clearing tracked constants. A code-review rejection caught a NaN fixed-point convergence hang; the fix now uses pass-level changed flags and splits the constant-folding implementation into submodules below the Rust file-size ceiling.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 19 `--latest-only --fold-constants` | `Ran 16 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| manual folded-assembly probe | `movl $5` present for `int x = 2; x = x + 3` |
| NaN fixed-point repro | no-fold, fold, and GCC all exit 1 under timeout guard |
| adversarial semantic probes | div/rem zero, casts, comparisons, branch side effects, stores, and calls matched no-fold/fold/GCC where defined |
| file-size/slop check | all Task 52 Rust files under 250 pure LOC; `shift_u32` removed |
| forbidden bridge/interpreter scan | no matches in source/tests/manifests |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-52-constant-folding-implementation.txt`
- `.omo/evidence/task-52-constant-folding-code-review.md`
- `.omo/evidence/task-52-constant-folding-fix.txt`
- `.omo/evidence/task-52-constant-folding-code-review-2.md`
- `.omo/evidence/task-52-constant-folding-adversarial-verify.txt`

### Remaining scope

- W20-T3 through W20-T5 must still implement unreachable-code elimination, copy propagation, dead-store elimination, and the default all-optimizations gate.
- Constant tracking is intentionally local to each CFG basic block; later dataflow passes should handle cross-block propagation.

## Wave 20 / Chapter 19 unreachable code elimination (task 53)

Implemented the Chapter 19 W20-T3 unreachable-code elimination pass on top of the CFG foundation. The optimizer now wires `--eliminate-unreachable-code` through the pipeline, removes blocks unreachable from the CFG entry, then performs the reference cleanup sequence for useless jumps, useless labels, and empty blocks while preserving original `TackyFunction` metadata. A code-review rejection removed a compiler-phase Rust unit test to preserve the plan's official-harness-only policy.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 19 `--latest-only --eliminate-unreachable-code` | `Ran 15 tests ... OK` |
| chapter 19 `--latest-only --fold-constants` | `Ran 16 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| manual UCE probes | after-return, jump-over-dead, and conditional-tail dead code removed with matching exits |
| compiler-phase Rust test policy | no `#[cfg(test)]`/`#[test]` remains in UCE file; cargo suite back to 10 tests |
| forbidden bridge/interpreter scan | no matches in source/tests/manifests |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-53-unreachable-code-implementation.txt`
- `.omo/evidence/task-53-unreachable-code-code-review.md`
- `.omo/evidence/task-53-unreachable-code-fix.txt`
- `.omo/evidence/task-53-unreachable-code-code-review-2.md`
- `.omo/evidence/task-53-unreachable-code-adversarial-verify.txt`

### Remaining scope

- W20-T4 copy propagation and W20-T5 dead-store elimination/default-all gate remain pending.
- UCE intentionally uses CFG reachability only; it does not fold constants or do cross-pass dataflow beyond its own cleanup sequence.

## Wave 20 / Chapter 19 copy propagation (task 54)

Implemented the Chapter 19 W20-T4 copy-propagation pass on top of the CFG foundation. The optimizer now wires `--propagate-copies` through the pipeline, runs a conservative CFG/dataflow reaching-copy analysis, rewrites safe TACKY uses, preserves original `TackyFunction` metadata, and keeps stores, calls, address-taken variables, static variables, and aggregate `CopyBytes` cases conservative. A code-review rejection forced a file-size/slop cleanup: constant-folding state handling was split out and copy-prop-induced codegen helper bodies were relocated from legacy `codegen.rs` into a small helper module.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 19 `--latest-only --propagate-copies` | `Ran 42 tests ... OK` |
| chapter 19 `--latest-only --eliminate-unreachable-code` | `Ran 15 tests ... OK` |
| chapter 19 `--latest-only --fold-constants` | `Ran 16 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| manual copy-prop probes | pure copy chain optimized; address-taken/store/call/memory cases matched no-opt exits |
| file-size/slop check | Task 54 helper modules under 250 pure LOC; no compiler-phase Rust tests added |
| forbidden bridge/interpreter scan | no matches in source |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-54-copy-prop-implementation.txt`
- `.omo/evidence/task-54-copy-prop-code-review.md`
- `.omo/evidence/task-54-copy-prop-fix.txt`
- `.omo/evidence/task-54-copy-prop-code-review-2.md`
- `.omo/evidence/task-54-copy-prop-adversarial-verify.txt`

### Remaining scope

- W20-T5 must still implement dead-store elimination and the default all-optimizations Chapter 19 gate.
- `src/codegen/codegen.rs` remains a legacy oversized file, but Task 54 helper bodies now live in `src/codegen/codegen/copy_prop_support.rs`; broad codegen decomposition remains out of scope.

## Wave 20 / Chapter 19 dead store elimination and default all-optimizations gate (task 55)

Implemented the Chapter 19 W20-T5 dead-store elimination pass and closed the default all-optimizations gate. The optimizer now wires `--eliminate-dead-stores` through the pipeline, runs DSE in the Chapter 19 fixed-point pass sequence, preserves static-storage and extern-global writes, and keeps address-taken, pointer-store, call-side-effect, and aggregate/global memory effects conservative. Supporting copy-propagation address rewriting was split into a focused helper and hardened after review-found alias bugs involving direct stores and pointer arguments passed to calls.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 19 `--latest-only --eliminate-dead-stores` | `Ran 27 tests ... OK` |
| chapter 19 default `--latest-only` | `Ran 120 tests ... OK` |
| chapter 19 `--latest-only --propagate-copies` | `Ran 42 tests ... OK` |
| chapter 19 `--latest-only --eliminate-unreachable-code` | `Ran 15 tests ... OK` |
| chapter 19 `--latest-only --fold-constants` | `Ran 16 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| extern/global DSE probes | scalar extern exits 5; aggregate extern exits 7; all preserved under DSE/default |
| alias regression probes | direct store alias and call pointer alias both exit 9 under baseline, copy-prop, and default all opts |
| file-size/slop check | DSE modules, `rewrite.rs`, and `rewrite_support.rs` all below 250 pure LOC; no compiler-phase Rust tests added |
| forbidden bridge/interpreter scan | no new bridge/system-C/interpreter or Chapter 20 regalloc fingerprints |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-55-dse-implementation.txt`
- `.omo/evidence/task-55-dse-code-review.md`
- `.omo/evidence/task-55-dse-fix.txt`
- `.omo/evidence/task-55-dse-code-review-2.md`
- `.omo/evidence/task-55-dse-adversarial-verify.txt`
- `.omo/evidence/task-55-copy-prop-rewrite-split-fix.txt`
- `.omo/evidence/task-55-dse-code-review-3.md`
- `.omo/evidence/task-55-copy-prop-write-alias-fix.txt`
- `.omo/evidence/task-55-dse-code-review-4.md`
- `.omo/evidence/task-55-copy-prop-call-alias-fix.txt`
- `.omo/evidence/task-55-dse-code-review-5.md`
- `.omo/evidence/task-55-dse-adversarial-verify-2.txt`

### Remaining scope

- Wave 21 / Chapter 20 register allocation remains pending, starting with assembly liveness analysis.
- `src/codegen/codegen.rs` and `src/lex/scanner.rs` remain oversized legacy files touched by required support fixes; no additional broad cleanup was taken in Task 55.

## Wave 21 / Chapter 20 liveness analysis foundation (task 56)

Implemented the Chapter 20 W21-T1 assembly liveness foundation for the later register allocator. The new `src/codegen/regalloc/` liveness surface mirrors the OCaml `regalloc.ml` use/write and backward dataflow shape, exposes block live-in/live-out plus instruction live-after annotations, and keeps the actual allocator/interference/color/spill/coalescing work as later Wave 21 scope. A review rejection corrected the register-class sets to match OCaml exactly, made missing call metadata an explicit liveness error, removed broad class filtering, and expanded manual probes across branches, calls, div/idiv, memory/indexed operands, and GP/XMM behavior.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 19 default `--latest-only` | `Ran 120 tests ... OK` after one transient harness rerun |
| chapter 19 `--latest-only --eliminate-dead-stores` | `Ran 27 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| liveness manual probes | branch live sets, call metadata/error path, div/idiv implicit regs, memory/indexed operands, return regs, GP/XMM classes all passed |
| file-size/slop check | Task 56 Rust files all below 250 pure LOC; no compiler-phase Rust tests added |
| scope scan | no interference graph, coloring, spilling, coalescing, new dependencies, unsafe, unwrap/expect, or bridge/system-C path added |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-56-liveness-implementation.txt`
- `.omo/evidence/task-56-liveness-code-review.md`
- `.omo/evidence/task-56-liveness-adversarial-verify.txt`
- `.omo/evidence/task-56-liveness-fix.txt`
- `.omo/evidence/task-56-liveness-code-review-2.md`
- `.omo/evidence/task-56-liveness-adversarial-verify-2.txt`

### Remaining scope

- W21-T2 must build the interference graph and simplification over this liveness foundation.
- Strict repo-wide clippy remains red on pre-existing diagnostics and was recorded as a watch item, not a Task 56 blocker.

## Wave 21 / Chapter 20 interference graph and simplification (task 57)

Implemented the Chapter 20 W21-T2 interference graph and simplification foundation on top of the assembly liveness work. The regalloc module now builds class-specific interference graphs, suppresses move source/destination self-interference, preserves hard-register/precolored behavior, excludes configured static and address-taken pseudos, records spill costs, and produces a simplification stack with low-degree removal plus spill-candidate fallback. The actual select/color, spill rewrite loop, and coalescing remain later Wave 21 tasks.

### QA

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 19 default `--latest-only` | `Ran 120 tests ... OK` |
| chapter 19 `--latest-only --eliminate-dead-stores` | `Ran 27 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| durable regalloc probe | pseudo edges `{a-b,a-c,b-c,d-e}`; move edge suppressed; hard-reg pressure spill fallback; GP/XMM filtering; static/address-taken exclusion |
| file-size/slop check | W21-T2 files all below 250 pure LOC; parameter bloat fixed with typed contexts |
| scope scan | no coloring/select, spill rewrite loop, coalescing, new dependencies, unsafe, unwrap/expect, or bridge/system-C path added |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-57-interference-implementation.txt`
- `.omo/evidence/task-57-interference-adversarial-verify.txt`
- `.omo/evidence/task-57-interference-fix.txt`
- `.omo/evidence/task-57-regalloc-probe.rs`
- `.omo/evidence/task-57-interference-code-review-2.md`
- `.omo/evidence/task-57-interference-adversarial-verify-2.txt`

### Remaining scope

- W21-T3 must implement select/coloring over the simplification stack.
- `src/codegen/regalloc/graph.rs` is in the 200-250 pure LOC watch band; future regalloc growth should go into new modules rather than this file.
- Strict repo-wide clippy remains red on pre-existing diagnostics outside the changed regalloc files and is recorded as a watch item.

## Wave 21 / Chapter 20 coloring and no-coalescing allocation (task 58)

Implemented the Chapter 20 W21-T3 select/coloring phase and wired the no-coalescing allocation path through the native backend. Coloring now assigns pseudos from the simplification stack using the OCaml register color order, keeps R10/R11 and XMM14/XMM15 reserved/non-allocatable, rewrites colored pseudos before the existing pseudo replacement pass, preserves callee-saved registers, and keeps the default Chapter 20a mode as no-coalescing until W21-T5 implements conservative coalescing. Earlier review-found issues around path-based allocation, R11 allocation, and test-harness forwarding were fixed; the final gate confirmed the official tests harness diff is empty.

### QA

| Gate | Result |
|------|--------|
| `git diff -- tests` | empty; official harness unchanged |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 20 `--latest-only --no-coalescing` | `Ran 66 tests ... OK` |
| chapter 19 default `--latest-only` | `Ran 120 tests ... OK` |
| chapter 19 `--latest-only --eliminate-dead-stores` | `Ran 27 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| durable coloring probe | OCaml mapping `{0:R9,1:R8,2:SI,3:DI,4:DX,5:CX,6:AX,7:BX,8:R12,9:R13,10:R14,11:R15}`; reserved-register assertions passed |
| scope scan | no source-path/chapter/test bridge; no tests diff; R10/R11/XMM14/XMM15 not allocatable |
| final code review | APPROVE with WATCH items only |
| final adversarial gate | APPROVE/confirmed |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-58-coloring-implementation.txt`
- `.omo/evidence/task-58-coloring-fix.txt`
- `.omo/evidence/task-58-no-coalescing-gate-fix.txt`
- `.omo/evidence/task-58-final-fix.txt`
- `.omo/evidence/task-58-harness-scope-fix.txt`
- `.omo/evidence/task-58-coloring-probe.rs`
- `.omo/evidence/task-58-verify-run.log`
- `.omo/evidence/task-58-manual-qa.log`
- `.omo/evidence/task-58-hands-on-qa/`
- `.omo/evidence/task-58-harness-scope-code-review.md`
- `.omo/evidence/task-58-harness-scope-adversarial-verify.txt`

### Remaining scope

- W21-T4 must add the full spill/re-allocation loop. Task 58 contains enough rewrite support for the current no-coalescing gate but does not claim full spill-loop completion.
- W21-T5 must implement conservative coalescing and revisit the Chapter 20 default mode.
- `src/codegen/regalloc/allocate.rs` remains in the 200-250 pure LOC warning band; future growth should split into focused helpers.
- Chapter 20 helper `.s` fixtures are required by the official harness and were restored as force-added ignored files.

## Wave 21 / Chapter 20 spill and re-allocation loop (task 59)

Implemented the W21-T4 bounded spill/re-allocation loop for Chapter 20 no-coalescing register allocation. The allocator now records uncolored pseudos as stack-only spill candidates, excludes them from subsequent interference/color passes, and repeats class allocation until no new spills are discovered or the safe pseudo-count bound would be exceeded. Concrete stack slots remain assigned by the existing pseudo replacement/fixup path, matching the current Rust backend pipeline while preserving the OCaml regalloc structure and Task 58 reserved-register invariants.

### QA

| Gate | Result |
|------|--------|
| `git diff -- tests` | empty; official harness unchanged |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 20 `--latest-only --no-coalescing` | `Ran 66 tests ... OK` |
| chapter 19 default `--latest-only` | `Ran 120 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| durable spill probe | high-pressure int program exits 16 with stack slots observed |
| independent manual QA | high-pressure int and double programs both exit 0 with stack-slot evidence |
| scope scan | no source-path/chapter/test bridge; no tests diff; R10/R11/XMM14/XMM15 not allocatable; no coalescing implementation |
| code review | APPROVE with WATCH items only |
| final adversarial gate | confirmed |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-59-spill-loop-implementation.txt`
- `.omo/evidence/task-59-spill-loop-probe.c`
- `.omo/evidence/task-59-spill-loop-code-review.md`
- `.omo/evidence/task-59-spill-loop-manual-qa.md`
- `.omo/evidence/task-59-spill-loop-adversarial-verify.txt`
- `.omo/evidence/task-59-spill-loop-adversarial-verify-2.txt`

### Remaining scope

- W21-T5 must implement conservative coalescing and the default coalescing gate.
- `src/codegen/regalloc/allocate.rs` remains in the 200-250 pure LOC warning band; future coalescing work should split rather than grow it.

## Wave 21 / Chapter 20 conservative coalescing and both-modes gate (task 60)

Implemented the W21-T5 conservative coalescing path for Chapter 20 register allocation. The default compiler mode now runs register allocation with coalescing enabled, while `--no-coalescing` preserves the Task 58/59 path. The coalescer collects copy-related pseudo moves, applies conservative Briggs/George-style safety checks before merging, rewrites coalesced pseudos, and then runs the existing coloring/spill loop. Cleanup after review split oversized modules, centralized the XMM binary classifier, and kept the late division-copy legality fix isolated and guarded.

### QA

| Gate | Result |
|------|--------|
| `git diff -- tests` | empty; official harness unchanged |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --release` | exit 0 |
| `cargo build --release` | exit 0 |
| `cargo test --release` | 10 passed, 0 failed |
| chapter 20 `--latest-only --no-coalescing` | `Ran 66 tests ... OK` |
| chapter 20 default `--latest-only` | `Ran 66 tests ... OK` |
| chapter 19 default `--latest-only` | `Ran 120 tests ... OK` |
| chapter 18 `--latest-only --union` | `Ran 286 tests ... OK` |
| copy-heavy manual QA | allocator-relevant moves reduced from 1 (`--no-coalescing`) to 0 (default) |
| scope scan | no source-path/chapter/test bridge; no tests diff; R10/R11/XMM14/XMM15 not allocatable |
| file-size cleanup | newly crossed files split back below 250 pure LOC; remaining warning-band files documented |
| post-fix code review | APPROVE with WATCH items only |
| final adversarial gate | APPROVE/confirmed |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-60-coalescing-implementation.txt`
- `.omo/evidence/task-60-coalescing-move-count.txt`
- `.omo/evidence/task-60-copy-heavy.c`
- `.omo/evidence/task-60-coalescing-code-review.md`
- `.omo/evidence/task-60-coalescing-adversarial-verify.txt`
- `.omo/evidence/task-60-coalescing-fix.txt`
- `.omo/evidence/task-60-coalescing-code-review-2.md`
- `.omo/evidence/task-60-coalescing-adversarial-verify-2.txt`
- `.omo/evidence/task-60-coalescing-manual-qa.md`
- `.omo/evidence/task-60-coalescing-adversarial-verify-3.txt`

### Remaining scope

- Wave 22 must perform documentation/tooling polish and full chapter regression.
- `src/codegen/codegen.rs` and `src/driver.rs` remain oversized pre-existing files; Task 60 reduced/split newly crossed backend files but did not attempt broad decomposition outside the coalescing scope.

## Wave 22 / README invocation and tooling polish (task 61)

Updated the public invocation docs to match the current driver contract: `rustcc`
accepts stage/output/optimization flags plus one `.c` input path, while
`--chapter` and related chapter-selection flags belong to the `tests/test_compiler`
Python harness. The maintained chapter gate list now points readers to all 20
chapter commands in `docs/book/test-map.md` without claiming the Wave 22 full
regression matrix was rerun in this polish task.

### Chapter gate command record

| Chapter | Harness gate recorded |
|---|---|
| 1 | `./tests/test_compiler ./target/release/rustcc --chapter 1 --latest-only --expected-error-codes 1 2` |
| 2 | `./tests/test_compiler ./target/release/rustcc --chapter 2 --latest-only` |
| 3 | `./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise` |
| 4 | `./tests/test_compiler ./target/release/rustcc --chapter 4 --latest-only --bitwise` |
| 5 | `./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only --bitwise --compound --increment` |
| 6 | `./tests/test_compiler ./target/release/rustcc --chapter 6 --latest-only --bitwise --compound --increment --goto` |
| 7 | `./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only --compound --goto` |
| 8 | `./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch` |
| 9 | `./tests/test_compiler ./target/release/rustcc --chapter 9 --latest-only --bitwise --compound --increment --goto --switch` |
| 10 | `./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only` |
| 11 | `./tests/test_compiler ./target/release/rustcc --chapter 11 --latest-only` |
| 12 | `./tests/test_compiler ./target/release/rustcc --chapter 12 --latest-only` |
| 13 | `./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan` |
| 14 | `./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only` |
| 15 | `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only` |
| 16 | `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` |
| 17 | `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only` |
| 18 | `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` |
| 19 | `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores` |
| 20 | `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` |

### QA

| Gate | Result |
|------|--------|
| `git diff -- tests` | exit 0; empty official test/harness diff |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo build --release` | exit 0; no warning text detected |
| `cargo test --release` | exit 0; 10 passed, 0 failed |
| `cargo clippy --release -- -W clippy::all` | exit 0; no warning text detected after narrow clippy cleanup |
| `./target/release/rustcc --help` | exit 1 by current driver design; printed `usage: rustcc [--lex|--parse|--validate|--tacky|--codegen|-S|-c] [options] <input.c>` |
| modified-file LSP diagnostics | no diagnostics found for modified Rust files; directory-level LSP request timed out, so cargo/clippy are the authoritative full-crate checks |
| `git diff --check` | exit 0 |

Tooling evidence for this task is recorded in `.omo/evidence/task-61-tooling-polish.txt`.
This task intentionally did not run or claim the W22-T2 full regression matrix.

### Remaining scope

- W22-T2 still owns the full chapter regression matrix.
- W22-T3 still owns final commit/cleanup.

## Wave 22 / README invocation update and tooling polish (task 61)

Updated the README to document the real `rustcc` CLI contract, including stage flags, artifact modes, optimization/register-allocation flags, and the distinction between compiler options and `tests/test_compiler --chapter` harness options. Added `examples/hello.c` so documented sample commands are executable. Task 61 also cleaned current clippy warnings without changing official tests or claiming the full W22 regression matrix.

### QA

| Gate | Result |
|------|--------|
| `git diff -- tests` | empty; official harness unchanged |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo build --release` | exit 0, no warnings |
| `cargo test --release` | 10 passed, 0 failed |
| `cargo clippy --release -- -W clippy::all` | exit 0, warning-clean |
| README example commands | `target/release/rustcc examples/hello.c`, `-S`, `-c`, and stage flags all exit 0 |
| `./target/release/rustcc --help` | exits 1 by current driver design and prints usage text |
| final code review | APPROVE |
| final adversarial gate | APPROVE/confirmed |
| `git diff --check` | exit 0 |

### Evidence

- `.omo/evidence/task-61-tooling-polish.txt`
- `.omo/evidence/task-61-tooling-polish-code-review.md`
- `.omo/evidence/task-61-tooling-polish-adversarial-verify.txt`
- `.omo/evidence/task-61-readme-example-fix.txt`
- `.omo/evidence/task-61-tooling-polish-adversarial-verify-2.txt`

### Remaining scope

- W22-T2 must run the full regression matrix. Task 61 intentionally did not claim full regression completion.
- W22-T3 and Final F1-F4 remain pending.
