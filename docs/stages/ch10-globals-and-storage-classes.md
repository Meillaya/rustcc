# Chapter 10 — Globals and Storage Classes

## Purpose

Add file-scope variables, static/extern storage classes, linkage, tentative definitions, and static locals.

## Parser pseudocode

```text
parse_declaration_specifiers:
  collect storage class: static | extern | none
  collect type specifier

parse_file_scope_item:
  function definition | function declaration | variable declaration/definition
```

## Semantic pseudocode

```text
global symbol table entry:
  name, type, linkage, storage duration, defined?, initializer?

merge declarations:
  extern declaration can refer to prior definition
  static has internal linkage
  reject incompatible linkage/type/redefinition

tentative definition:
  if no initializer and no full definition appears, emit zero initialization
```

## Codegen pseudocode

```text
emit initialized global:
  data section with value

emit tentative/uninitialized global:
  bss/zero storage directive

static local:
  allocate static storage with unique internal name
  block-scope name resolves to static symbol
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 10 valid/invalid/helper-library tests.
