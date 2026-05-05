# Chapter 15 — Arrays and Pointer Arithmetic

## Purpose

Add arrays, array-to-pointer decay, subscripting, and scaled pointer arithmetic.

## Parser pseudocode

```text
parse declarator suffix:
  [ constant_size ] wraps declarator type in Array(element, count)

parse postfix:
  expr [ index_expr ] becomes Subscript(expr, index_expr)
```

## Semantic pseudocode

```text
array expression decay:
  in most value contexts, array T[N] converts to pointer_to(T)
  exceptions include sizeof/address contexts as required by active subset

pointer arithmetic:
  pointer + integer -> pointer advanced by integer * sizeof(pointed type)
  pointer - integer -> pointer retreated by scaled amount
  pointer - pointer -> element distance when compatible

subscript:
  a[i] == *(a + i)
```

## Codegen pseudocode

```text
array local storage reserves contiguous element slots
array indexing computes base_address + index * element_size
loads/stores use computed element address
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 15 valid/invalid tests.
