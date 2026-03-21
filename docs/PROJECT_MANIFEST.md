# rustcc — Project Manifesto

## Goal

Implement a **complete C compiler** based on Nora Sandler's book "Writing a C Compiler." Every feature in the book will be implemented.

## Guiding Principles

1. **Concept-first, code-second**: I will NOT simply write code for you. Instead, I will thoroughly explain the concepts, algorithms, and design decisions. When I create files, they will be heavily commented to document the "why" behind each implementation choice.

2. **Documentation as we go**: Every major component will have corresponding documentation in `docs/` explaining:
   - What the component does
   - How it works conceptually
   - Why it was implemented the way it was
   - How it fits into the broader compilation pipeline

3. **Scaffold then implement**: For each chapter/feature, I'll first create the scaffolding (types, structs, interfaces), document what's needed, then we'll implement step by step.

## What to Expect

- **I will explain**: Compiler theory, data structures, algorithms, assembly conventions, etc.
- **I will create files**: Project structure, type definitions, scaffolding
- **I will NOT do**: Write the full implementation without explanation
- **You will implement**: With my guidance, you write the actual code (or I write it with heavy commenting explaining every line)

## Progress Tracking

Each major milestone will be documented:
- `docs/stages/01-lexical-analysis.md`
- `docs/stages/02-parsing.md`
- `docs/stages/03-ast.md`
- etc.

---

## Book Overview (for reference)

Nora Sandler's "Writing a C Compiler" covers:

1. **Lexical Analysis (Lexer)** — Tokenizing input source
2. **Parsing** — Building an AST from tokens
3. **Semantic Analysis** — Type checking, scope resolution
4. **Code Generation** — Translating AST to assembly
5. **Runtime Support** — Implementing C's runtime semantics

The book implements a subset of C (roughly C11 without floats, structs, arrays initially), progressively adding features.

---

## Current Status

- [x] Project scaffold created (Rust)
- [ ] Lexical Analysis (Chapter 1)
- [ ] Parsing (Chapter 2)
- [ ] AST Representation (Chapter 3)
- [ ] Semantic Analysis
- [ ] Code Generation (x86-64)
- [ ] Runtime/Standard Library

---

## Communication Pattern

When you want to work on the compiler:

1. Tell me which chapter/section you're on
2. I'll explain the concepts
3. We'll create/modify files together
4. I'll document progress in `docs/`

Let's begin when you're ready!
