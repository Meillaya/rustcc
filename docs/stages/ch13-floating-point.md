# Chapter 13 — Floating Point Numbers

## Purpose

Add `double`, floating constants, conversions, and SSE/XMM ABI behavior.

## Lexer/parser pseudocode

```text
recognize floating constants with decimal/exponent forms required by tests
parse double type specifier
include casts between arithmetic scalar types
```

## Semantic pseudocode

```text
usual_arithmetic_conversions:
  if either operand is double, convert both to double
  otherwise use integer conversion rules

casts:
  signed/unsigned integer <-> double
  double -> integer follows target conversion behavior used by book/tests
```

## Codegen pseudocode

```text
double values live in XMM registers
floating constants emitted in rodata
floating arithmetic uses SSE instructions
floating comparisons normalize to int 0/1
function args/returns classify double values into XMM ABI registers
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 13 tests including helper libraries and `-lm`/optional `--nan` only when implemented.
