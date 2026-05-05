# Chapter 14 — Pointers

## Purpose

Add pointer types, address-of, dereference, pointer comparisons, and pointer casts.

## Parser pseudocode

```text
parse declarator:
  consume leading * to wrap base type in pointer layers

parse unary:
  & unary_expr
  * unary_expr
```

## Semantic pseudocode

```text
&expr:
  expr must be lvalue
  result type pointer_to(expr.type)

*expr:
  expr type must be pointer
  result type pointed_to(expr.type)
  result is lvalue

null pointer constants and compatible pointer comparisons follow active chapter rules
```

## Codegen pseudocode

```text
address-of local/global:
  produce address rather than value

dereference read:
  evaluate pointer address
  load pointed type size

dereference assignment:
  evaluate address then store rhs
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 14 valid/invalid tests.
