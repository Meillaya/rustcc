# Chapter 6 — If Statements and Conditional Expressions

## Purpose

Add `if`/`else` statements and ternary conditional expressions.

## Parser pseudocode

```text
parse_statement:
  if next is if:
    parse_if_statement()
  else if next is return:
    parse_return()
  else:
    parse expression/null statement

parse_if_statement:
  expect if ( expr )
  then_stmt = parse_statement()
  optional else_stmt = parse_statement() if next is else

parse_conditional_expr:
  condition = parse_logical_or()
  if next is ?:
    then_expr = parse_expr()
    expect :
    else_expr = parse_conditional_expr()
    return Conditional(condition, then_expr, else_expr)
  return condition
```

## Lowering pseudocode

```text
lower if:
  L_else = fresh label
  L_end = fresh label
  branch condition false -> L_else
  lower then
  jump L_end
  label L_else
  lower else if present
  label L_end

lower ternary:
  evaluate condition
  branch to then/else labels
  assign selected branch value into result temp
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 6 valid and invalid suites with relevant extra-credit flags when implemented.
