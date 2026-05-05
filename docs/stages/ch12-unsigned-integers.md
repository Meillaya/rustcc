# Chapter 12 — Unsigned Integers

## Purpose

Add unsigned integer types and signed/unsigned conversion rules.

## Type pseudocode

```text
Type includes:
  int, long, unsigned int, unsigned long
properties:
  width, signedness, rank
```

## Semantic pseudocode

```text
usual_arithmetic_conversions(a, b):
  if same signedness: choose greater rank
  if unsigned rank >= signed rank: convert signed to unsigned type
  if signed type can represent all unsigned values: convert unsigned to signed
  otherwise convert both to unsigned version of signed type
```

## Codegen pseudocode

```text
unsigned comparisons use unsigned condition codes
signed comparisons use signed condition codes
zero-extension for unsigned widening
truncation for narrowing
unsigned division/remainder use unsigned machine instructions
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 12 valid/invalid tests.
