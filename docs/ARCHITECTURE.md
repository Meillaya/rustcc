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

## References

- Nora Sandler's original blog series: https://norasandler.com/2017/11/29/Write-a-Compiler.html
- The book implements a subset of C11, starting with:
  - `int` only (no floats, no chars initially)
  - No structs, unions, arrays
  - Basic control flow: `if/else`, `while`
  - Functions (no recursion initially)
  - Gradually adds more features

We'll follow the same progression.
