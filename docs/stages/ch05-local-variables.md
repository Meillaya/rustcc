# Chapter 5 — Local Variables and Assignment

## Purpose

Add block-scope local declarations, variable references, assignments, and lvalue validation.

## Parser pseudocode

```text
parse_block_item:
  if next token begins a declaration:
    parse_declaration()
  else:
    parse_statement()

parse_declaration:
  expect int
  name = expect identifier
  optional initializer after =
  expect ;

parse_expr:
  include assignment as low-precedence right-associative expression
  assignment target must be syntactically an lvalue candidate
```

## Semantic pseudocode

```text
resolve_function_body:
  create scope stack
  for each declaration:
    reject duplicate in current scope
    assign unique internal symbol id
  for each variable use:
    lookup nearest visible declaration
    reject if missing

validate_assignment:
  left side must be variable or later lvalue expression
```

## Codegen pseudocode

```text
for each local variable:
  assign stack slot offset from base pointer

lower declaration with initializer:
  evaluate initializer
  store result in variable slot

lower variable read:
  load from variable slot

lower assignment:
  evaluate rhs
  store into lhs slot
  expression value is rhs value
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 5 valid, invalid_parse, invalid_semantics, and relevant extra-credit gates.
