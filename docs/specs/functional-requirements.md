# Functional Requirements

## Driver and CLI

- `FR-DRV-01`: The compiler SHALL accept a primary C source file input.
- `FR-DRV-02`: The compiler SHALL support stage-limited execution matching the test harness stages.
- `FR-DRV-03`: The compiler SHALL reject malformed invocations with a non-zero exit code.
- `FR-DRV-04`: The compiler SHALL derive deterministic output paths from the input stem.
- `FR-DRV-05`: The compiler SHALL not leave stale output artifacts after rejection.

## Lexing

- `FR-LX-01`: The lexer SHALL recognize all tokens needed by each active chapter.
- `FR-LX-02`: The lexer SHALL distinguish reserved words from identifiers.
- `FR-LX-03`: The lexer SHALL reject malformed token sequences covered by invalid lexical tests.
- `FR-LX-04`: The lexer SHOULD preserve source-position metadata for diagnostics.

## Parsing

- `FR-PRS-01`: The parser SHALL build an AST for all syntactically valid source programs in the supported subset.
- `FR-PRS-02`: The parser SHALL enforce operator precedence and associativity correctly.
- `FR-PRS-03`: The parser SHALL reject syntactically invalid programs at parse stage.
- `FR-PRS-04`: The parser SHALL support declarations, statements, expressions, and definitions as introduced chapter by chapter.

## Semantic analysis

- `FR-SEM-01`: The compiler SHALL resolve names according to scope rules.
- `FR-SEM-02`: The compiler SHALL enforce typing and conversion rules for the active chapter set.
- `FR-SEM-03`: The compiler SHALL reject invalid lvalues, declarations, labels, and type uses when required by the tests.
- `FR-SEM-04`: The compiler SHALL preserve enough type/layout information for lowering and backend work.

## IR and lowering

- `FR-IR-01`: The compiler SHALL lower validated programs into an IR suitable for control flow, optimization, and backend translation.
- `FR-IR-02`: The IR SHALL make side effects and control flow explicit.
- `FR-IR-03`: Lowering SHALL preserve source-level evaluation order where required.

## Code generation

- `FR-CODE-01`: The backend SHALL emit assembler-readable x86-64 code for valid programs.
- `FR-CODE-02`: The backend SHALL implement correct arithmetic, control flow, memory access, and call/return behavior for active features.
- `FR-CODE-03`: The backend SHALL support helper-library and multi-file tests used by the suite.

## Optimization and regalloc

- `FR-OPT-01`: The compiler SHALL implement the optimization passes required by chapter 19.
- `FR-OPT-02`: All optimizations SHALL preserve observable semantics.
- `FR-REG-01`: The compiler SHALL compute liveness/interference or an equivalent basis for register allocation.
- `FR-REG-02`: The allocator SHALL spill when register pressure exceeds available registers.
- `FR-REG-03`: The allocator SHALL preserve ABI and calling-convention constraints.
