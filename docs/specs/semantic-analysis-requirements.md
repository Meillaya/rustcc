# Semantic Analysis Requirements

## Scope and names

- identifiers SHALL resolve by lexical scope
- shadowing SHALL behave correctly
- file-scope and block-scope rules SHALL remain distinct

## Type rules

- assignment compatibility SHALL be enforced
- operator typing SHALL be enforced
- conversion rules SHALL be explicit for signed/unsigned, float/int, and pointer cases
- invalid lvalue use SHALL be rejected

## Layout and completeness

- arrays, strings, structs, and unions SHALL carry enough layout information for `sizeof`, addressing, and ABI behavior
- incomplete-type rules SHALL be enforced where relevant

## Control-flow validity

- `break` and `continue` SHALL only appear in valid contexts
- return statements SHALL match function result type expectations
