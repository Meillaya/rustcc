# Chapter 8 — Loops, Break, Continue

## Purpose

Add `while`, `do while`, `for`, `break`, and `continue`.

## Parser pseudocode

```text
parse_statement:
  while_statement | do_while_statement | for_statement | break ; | continue ; | existing statements

parse_for:
  expect for (
  init = declaration | expression? ;
  condition = expression? ;
  post = expression?
  expect )
  body = statement
```

## Semantic pseudocode

```text
loop_context_stack:
  push {break_label, continue_label} while resolving/lowering loop body
  break/continue valid only when stack is non-empty
  for-loop init declaration creates scope covering condition/post/body
```

## Lowering pseudocode

```text
while:
  label condition
  if condition false jump break
  lower body
  label continue
  jump condition
  label break

do while:
  label body
  lower body
  label continue
  if condition true jump body
  label break

for:
  lower init
  label condition
  if condition present and false jump break
  lower body
  label continue
  lower post
  jump condition
  label break
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 8 valid and invalid tests, plus optional switch/goto/compound/increment flags only when implemented.
