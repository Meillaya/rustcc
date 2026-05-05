# Chapter 3 — Binary Operators and Precedence

## Purpose

Chapter 3 extends return expressions with left-associative binary operators and a precedence-climbing parser. The current implementation still folds constant-only expressions before emitting assembly; later chapters can lower the expression tree into real TACKY instructions.

## Lexer pseudocode

```text
add tokens:
  + - * / %
  extra credit: & | ^ << >>
when seeing /:
  if // or /*, keep comment behavior
  otherwise emit Slash token
when seeing < or >:
  require doubled << or >> for the Chapter 3 extra-credit shift operators
```

## Parser pseudocode

```text
parse_expr(min_precedence):
  left = parse_unary_expr()
  while next token is a binary operator with precedence >= min_precedence:
    op = consume operator
    right = parse_expr(precedence(op) + 1)   # left associativity
    left = Binary(op, left, right)
  return left

precedence high to low:
  * / %
  + -
  << >>
  &
  ^
  |
```

## Evaluation/codegen pseudocode

```text
eval(Binary op left right):
  evaluate children as i32
  use wrapping add/sub/mul for Chapter 3 integer behavior
  use Rust signed / and % for C-like truncation toward zero
  use arithmetic right shift for signed >>

emit movl $eval(return_expr), %eax
emit ret
```

## Verification target

This chapter document is pseudocode guidance only. After you fill in the real Rust code, use the chapter-specific commands from `docs/book/test-map.md` and `.omx/plans/full-implementation-pseudocode-plan.md` to record actual evidence.
