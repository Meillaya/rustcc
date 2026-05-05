# Chapter 7 — Compound Statements and Nested Scopes

## Purpose

Add compound statements `{ ... }`, nested lexical scopes, and shadowing behavior.

## Parser pseudocode

```text
parse_statement:
  if next is {:
    parse_compound_statement()

parse_compound_statement:
  expect {
  items = []
  while next is not }:
    items.push(parse_block_item())
  expect }
```

## Semantic pseudocode

```text
resolve_block(block):
  push new scope
  resolve each block item in order
  pop scope

on declaration:
  reject duplicate in current scope
  allow shadowing declarations in nested child scopes
```

## Lowering/codegen pseudocode

```text
scope ids are resolved before lowering
stack slots may be assigned per unique symbol id
shadowed variables are distinct slots/symbols
compound statement lowers to each item in sequence
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 7 valid and invalid parse/semantic tests.
