# Compiler Architecture Overview

## The Compilation Pipeline

A C compiler typically works in **phases**:

```
Source Code (.c)
     │
     ▼
┌─────────────────┐
│ 1. Lexer        │  ← Breaks input into tokens
│ (Tokenization)  │
└────────┬────────┘
         │ Tokens
         ▼
┌─────────────────┐
│ 2. Parser       │  ← Organizes tokens into AST
│ (Syntax Anal.)  │
└────────┬────────┘
         │ AST (Abstract Syntax Tree)
         ▼
┌─────────────────┐
│ 3. Semantic    │  ← Type checking, scope
│    Analyzer    │    resolution, validation
└────────┬────────┘
         │ Annotated AST
         ▼
┌─────────────────┐
│ 4. Code Gen    │  ← Converts AST to assembly
│ (Codegen)       │
└────────┬────────┘
         │ Assembly (.s)
         ▼
┌─────────────────┐
│ 5. Assembler   │  ← Converts assembly to object
│                 │    (usually external: gas, clang)
└────────┬────────┘
         │ Object File (.o)
         ▼
┌─────────────────┐
│ 6. Linker      │  ← Combines object files
│                 │    (usually external: ld)
└────────┬────────┘
         │
         ▼
   Executable
```

---

## Our Implementation

We will implement phases 1-4. Phases 5-6 will leverage external tools (gcc/ld or similar).

### Module Structure

```
src/
├── main.rs         # Entry point, CLI handling
├── lexer.rs        # Phase 1: Tokenization
├── tokens.rs       # Token type definitions
├── parser.rs       # Phase 2: Parsing into AST
├── ast.rs          # AST node definitions
├── semantic.rs     # Phase 3: Type checking (future)
├── codegen.rs      # Phase 4: Assembly generation
├── emission.rs     # Assembly output utilities
└── error.rs       # Error handling types
```

### Each Phase Explained

#### Phase 1: Lexical Analysis (Lexer)

**Purpose**: Read the raw source code character by character and group them into meaningful units called **tokens**.

**Concept**: 
- The lexer scans left-to-right, looking for patterns (identifiers, numbers, symbols)
- Each token has a type (e.g., `IDENTIFIER`, `NUMBER`, `PLUS`, `IF`) and a value
- Whitespace and comments are discarded

**Why separate from parsing?** 
- Simplifies the parser — it doesn't worry about "is this `while` a keyword or an identifier?"
- Single-responsibility principle

**Output**: A stream of `Token` structs.

---

#### Phase 2: Parsing

**Purpose**: Organize the token stream into a tree structure that represents the program's grammatical structure.

**Concept**:
- The parser checks if the token stream follows C's grammar rules
- If valid, it builds an **Abstract Syntax Tree (AST)**
- Uses **recursive descent** or **parser generators** (we'll likely use recursive descent per Nora's book)

**Why recursive descent?**
- Intuitive: each grammar rule becomes a function
- Easy to debug and understand
- No external parser generator needed

**Output**: An `AST` (tree of nodes).

---

#### Phase 3: Semantic Analysis

**Purpose**: Check that the program makes logical sense beyond just syntax.

**Concept**:
- **Type checking**: Ensure operations make sense (can't add `int*` and `char`)
- **Scope resolution**: Variables must be declared before use
- **Symbol table**: Track what variables exist in what scope

**This is where many bugs are caught!**

---

#### Phase 4: Code Generation

**Purpose**: Translate the AST into assembly language (x86-64 for this book).

**Concept**:
- Walk the AST recursively
- For each node, emit corresponding assembly instructions
- Need to understand:
  - Register allocation
  - Stack frame layout
  - Calling conventions (cdecl/System V)
  - Assembly syntax (we'll use NASM/GAS style)

**Target**: x86-64 Linux (System V ABI)

---

## Key Data Structures We'll Build

### Tokens (Phase 1)

```rust
enum Token {
    Number(i64),
    Ident(String),
    // Keywords
    If, Else, While, Return, Int, ...
    // Symbols
    Plus, Minus, Star, Slash, ...
    // Special
    Eof,
}
```

### AST Nodes (Phase 2)

```rust
enum Expr {
    Number(i64),
    Ident(String),
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    // ...
}

enum Stmt {
    Return(Expr),
    Expr(Expr),
    If { cond: Expr, then_branch: Vec<Stmt>, else_branch: Option<Vec<Stmt>> },
    // ...
}
```

---

## Project Structure

```
nqcc2/           # OCaml reference implementation (Nora Sandler's)
  lib/
    lex.ml         # Lexer
    tokens.ml      # Token types
    parse.ml      # Parser
    ast.ml         # AST definitions
    tacky*.ml      # Tacky IR (intermediate representation)
    semantic_analysis/
      resolve.ml     # Identifier resolution
      typecheck.ml  # Type checking
      label_loops.ml # Loop/break/continue annotation
    backend/
      codegen.ml    # x86-64 code generation
      regalloc.ml   # Register allocation
    optimizations/  # Optimization passes
    assembly.ml     # Assembly AST
    emit.ml        # Assembly emission
  bin/
    main.ml        # Entry point
  test/            # Unit tests

tests/           # Official test suite (nlsandler/writing-a-c-compiler-tests)
  test_compiler   # Python test runner
  tests/chapter_X/# Test cases by chapter
```

## Compilation Pipeline (from nqcc2)

```rustcc
Source (.c)
    │
    ▼
1. Lexer (lex.ml)
    │ Token stream
    ▼
2. Parser (parse.ml) → AST (ast.ml)
    │ Untyped AST
    ▼
3. Semantic Analysis
    ├─ Resolve (resolve.ml)       → Resolved AST
    ├─ Label Loops (label_loops)  → Annotated AST  
    └─ Typecheck (typecheck.ml)   → Typed AST
    │
    ▼
4. Tacky Gen (tacky_gen.ml) → TACKY IR
    │
    ▼
5. Optimizations (optimizations/)
    ├─ Constant Folding
    ├─ Copy Propagation
    ├─ Dead Store Elimination
    └─ Unreachable Code Elimination
    │
    ▼
6. Backend (backend/)
    ├─ Codegen (codegen.ml)        → Assembly AST
    ├─ Address Taken Analysis
    ├─ Register Allocation
    ├─ Replace Pseudos
    └─ Instruction Fixup
    │
    ▼
7. Emit (emit.ml) → Assembly (.s)
```

## Our Rust Implementation

We'll mirror the OCaml structure:

| OCaml Module | Rust Module | Purpose |
|---|---|---|
| `lex.ml` | `lexer.rs` | Tokenize input |
| `tokens.ml` | `tokens.rs` | Token type definitions |
| `parse.ml` | `parser.rs` | Build AST |
| `ast.ml` | `ast.rs` | AST node types |
| `semantic_analysis/` | `semantic.rs` | Type checking, resolution |
| `tacky*.ml` | (embedded) | IR (we may skip or simplify) |
| `backend/` | `codegen.rs` | x86-64 code generation |
| `emit.ml` | `emission.rs` | Write assembly output |

## References

- [Nora Sandler's Blog](https://norasandler.com/2017/11/29/Write-a-Compiler.html): Original blog series
- [nqcc2](https://github.com/nlsandler/nqcc2): OCaml reference implementation (in `nqcc2/` folder)
- [Writing a C Compiler Tests](https://github.com/nlsandler/writing-a-c-compiler-tests): Official test suite (in `tests/` folder)
