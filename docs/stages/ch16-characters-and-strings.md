# Chapter 16 — Characters and Strings

## Purpose

Add `char`, character literals, string literals, byte-sized storage, and string data emission.

## Lexer/parser pseudocode

```text
recognize char constants including required escapes
recognize string literals and concatenate adjacent strings if required by tests
parse char type specifier
```

## Semantic pseudocode

```text
char is integer type with byte size
character constants have integer value
string literal has array-of-char storage with trailing null byte
string expression decays to pointer to first char in value contexts
```

## Codegen pseudocode

```text
byte loads/stores for char objects
sign/zero extension follows char signedness chosen by project/book target
emit string literals in readonly data with unique labels
```

## Verification target

Pseudocode guidance only. After real code is filled in, run chapter 16 valid/invalid tests.
