# Chapter 9 — Functions and Calls

## Purpose

Add multiple function definitions/declarations, parameters, calls, and ABI basics.

## Parser pseudocode

```text
parse_translation_unit:
  while not EOF:
    parse declaration specifiers
    parse function declarator
    if next is { parse function body else expect ; declaration

parse_call_expr:
  primary '(' comma_separated_arguments? ')'
```

## Semantic pseudocode

```text
global_function_table:
  record name, parameter count/types, return type, definition/declaration status
  reject conflicting declarations
  reject duplicate definitions

resolve function body:
  parameters are declarations in function scope
  calls must match known function arity/type rules for active chapter
```

## Codegen pseudocode

```text
function prologue:
  establish stack frame
  move incoming ABI argument registers/stack args into parameter slots

call:
  evaluate arguments
  place first integer args in ABI registers
  push extra args right-to-left
  align stack before call
  emit call
  result is return register
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 9 valid/invalid and multi-function tests.
