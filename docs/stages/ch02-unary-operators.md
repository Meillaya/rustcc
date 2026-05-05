# Chapter 2 — Unary Operators

## Purpose

Chapter 2 extends the Chapter 1 return expression from a single integer constant to recursive unary expressions with parentheses.

## Lexer pseudocode

```text
add tokens:
  -  -> unary negation token
  ~  -> bitwise complement token
keep integer constants as signed 32-bit-range source constants for this subset
```

## Parser pseudocode

```text
parse_expr:
  if next is integer constant:
    return Constant(value)
  if next is -:
    consume -
    return Negate(parse_expr())
  if next is ~:
    consume ~
    return Complement(parse_expr())
  if next is (:
    consume (
    inner = parse_expr()
    expect )
    return inner
  otherwise reject parse error
```

## Evaluation/codegen pseudocode

The current compiler folds Chapter 2's constant-only expression tree before assembly emission:

```text
eval(Constant n) = n
eval(Negate e) = wrapping_neg(eval(e))
eval(Complement e) = bitwise_not(eval(e))

emit movl $eval(return_expr), %eax
emit ret
```

This preserves the observable behavior needed by the chapter tests while leaving later chapters free to lower expressions into real TACKY instructions.

## Verification target

This chapter document is pseudocode guidance only. After you fill in the real Rust code, use the chapter-specific commands from `docs/book/test-map.md` and `.omx/plans/full-implementation-pseudocode-plan.md` to record actual evidence.
