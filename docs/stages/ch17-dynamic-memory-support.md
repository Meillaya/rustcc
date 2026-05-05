# Chapter 17 — Sizeof, Void, and Dynamic Memory Support

## Purpose

Add `sizeof`, `void`, `void*`, and enough type/layout support for dynamic memory patterns in the tests.

## Parser pseudocode

```text
parse sizeof unary:
  sizeof expr
  sizeof ( type_name )

parse void type specifier
parse abstract declarators needed for sizeof(type) and casts
```

## Semantic pseudocode

```text
sizeof(expr):
  determine expression type without evaluating side effects when language rule requires
  result is size constant

sizeof(type):
  require complete object type where necessary

void:
  valid as function return or pointer target
  invalid as object type except allowed contexts

void* conversions:
  compatible with object pointers per active subset rules
```

## Codegen pseudocode

```text
sizeof lowers to integer constant
void-returning functions do not read return value
malloc/free are external calls using existing function-call ABI
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 17 valid/invalid tests.
