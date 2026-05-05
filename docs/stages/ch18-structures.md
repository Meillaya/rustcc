# Chapter 18 — Structures and Unions

## Purpose

Add struct/union tags, aggregate layout, member access, copying, initialization, and ABI treatment.

## Parser pseudocode

```text
parse struct_or_union_specifier:
  struct tag?
  optional { member_declarations }

parse member access:
  expr . identifier
  expr -> identifier
```

## Semantic pseudocode

```text
tag namespace:
  resolve struct/union tags separately from ordinary identifiers
  support incomplete declarations and later completion

layout struct:
  offset each member at required alignment
  size rounds up to aggregate alignment

layout union:
  all members offset 0
  size/alignment are max member size/alignment

member access:
  base must be aggregate or pointer-to-aggregate for arrow
  member must exist
```

## Codegen pseudocode

```text
member address = base address + member offset
struct copy copies size bytes
aggregate argument/return classification follows SysV AMD64 rules
memory-class returns use hidden return pointer
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 18 valid/invalid/helper tests and optional `--union` only when implemented.
