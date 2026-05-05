# Chapter 11 — Long Integers

## Purpose

Add `long` and width-aware integer semantics.

## Type pseudocode

```text
Type::Int  -> 32-bit signed
Type::Long -> 64-bit signed
integer constants infer type based on suffix/range rules required by tests
```

## Semantic pseudocode

```text
usual_integer_conversion(left, right):
  choose common type by rank
  convert narrower operand to wider type

assignment:
  convert rhs to lhs type
```

## Codegen pseudocode

```text
for int operations:
  use 32-bit registers/instructions
for long operations:
  use 64-bit registers/instructions
casts:
  int -> long sign extend
  long -> int truncate
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 11 valid/invalid tests.
