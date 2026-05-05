# Chapter 1 — Minimal Return-Only Compiler

## Purpose

Chapter 1 turns the Chapter 0 driver contract into the smallest working compiler accepted by the official test suite: a single `int <name>(void) { return <integer>; }` function.

## Lexer pseudocode

```text
while not end of source:
  skip whitespace
  skip // line comments
  skip /* block comments */ or reject if unterminated
  recognize keywords: int, void, return
  recognize identifiers: [A-Za-z_][A-Za-z0-9_]*
  recognize unsigned decimal integer constants
  reject digit-started identifier shapes like 1foo
  recognize punctuation: ( ) { } ;
  reject every other character as a lexical error
append EOF
```

## Parser pseudocode

```text
program:
  expect int
  function_name = expect identifier
  expect (
  expect void
  expect )
  expect {
  expect return
  value = expect integer_constant
  expect ;
  expect }
  expect EOF
  return Program(function_name, value)
```

## Codegen pseudocode

```text
emit .globl <function_name>
emit <function_name>:
emit movl $<return_value>, %eax
emit ret
```

## Verification target

This chapter document is pseudocode guidance only. After you fill in the real Rust code, use the chapter-specific commands from `docs/book/test-map.md` and `.omx/plans/full-implementation-pseudocode-plan.md` to record actual evidence.
