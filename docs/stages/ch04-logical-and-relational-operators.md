# Chapter 4 — Logical and Relational Operators

## Purpose

Add boolean-producing expressions, comparison operators, logical not, and short-circuiting logical `&&` / `||`.

## Lexer pseudocode

```text
add tokens:
  !
  && ||
  == !=
  < <= > >=
when reading !, <, >, =:
  prefer two-character tokens when the next character is =
  reject bare = until assignment is introduced later
```

## Parser pseudocode

```text
precedence high to low:
  unary: ! - ~
  * / %
  + -
  << >>
  < <= > >=
  == !=
  &
  ^
  |
  &&
  ||

parse_expr(min_precedence):
  left = parse_unary_expr()
  while next binary operator precedence >= min_precedence:
    op = consume operator
    right = parse_expr(precedence(op) + 1)
    left = Binary(op, left, right)
  return left
```

## Lowering/codegen pseudocode

```text
lower !expr:
  value = lower expr
  result = (value == 0) ? 1 : 0

lower comparison:
  evaluate left and right
  emit compare
  normalize result to integer 0 or 1

lower logical_and:
  evaluate left
  if left == 0: result = 0, skip right
  else: evaluate right; result = (right != 0)

lower logical_or:
  evaluate left
  if left != 0: result = 1, skip right
  else: evaluate right; result = (right != 0)
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 4 latest/cumulative tests, plus parse/codegen stage tests and optional bitwise flags as mapped in `docs/book/test-map.md`.
