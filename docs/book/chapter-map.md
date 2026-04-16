# Chapter Map

| Chapter | Main theme | Primary compiler areas | Primary test directories |
|---|---|---|---|
| 1 | Minimal compiler | driver, lexer, parser, direct codegen | `chapter_1/valid`, `chapter_1/invalid_lex`, `chapter_1/invalid_parse` |
| 2 | Unary operators | parser, AST, expression codegen | `chapter_2/valid`, `chapter_2/invalid_parse` |
| 3 | Binary operators | precedence handling, expression codegen | `chapter_3/valid`, `chapter_3/invalid_parse` |
| 4 | Logical/relational ops | control-flow lowering, boolean normalization | `chapter_4/valid`, `chapter_4/invalid_parse` |
| 5 | Local variables | environments, stack slots, assignment semantics | `chapter_5/valid`, `chapter_5/invalid_parse`, `chapter_5/invalid_semantics` |
| 6 | if / ternary | control-flow graph shape, branching | `chapter_6/valid`, `chapter_6/invalid_*` |
| 7 | Compound statements | nested scopes, shadowing | `chapter_7/valid`, `chapter_7/invalid_*` |
| 8 | Loops | loop labels, break/continue, backedges | `chapter_8/valid`, `chapter_8/invalid_*` |
| 9 | Functions | signatures, calls, ABI basics | `chapter_9/valid`, `chapter_9/invalid_*` |
| 10 | Globals/storage class | linkage, definitions, data emission | `chapter_10/valid`, `chapter_10/invalid_*` |
| 11 | Long integers | width-aware typing and codegen | `chapter_11/valid`, `chapter_11/invalid_*` |
| 12 | Unsigned integers | conversions, zero-extension, unsigned compares | `chapter_12/valid`, `chapter_12/invalid_*` |
| 13 | Floating point | SSE/XMM handling, conversions, helper libs | `chapter_13/valid`, `chapter_13/helper_libs`, `chapter_13/invalid_*` |
| 14 | Pointers | address/deref, pointer typing | `chapter_14/valid`, `chapter_14/invalid_*` |
| 15 | Arrays | layout, decay, pointer arithmetic | `chapter_15/valid`, `chapter_15/invalid_*` |
| 16 | Chars/strings | literal encoding, byte semantics | `chapter_16/valid`, `chapter_16/invalid_*` |
| 17 | Dynamic memory support | `sizeof`, `void`, `void*` | `chapter_17/valid`, `chapter_17/invalid_*` |
| 18 | Structures | aggregate layout, member access, ABI details | `chapter_18/valid`, `chapter_18/invalid_*` |
| 19 | Optimizations | TACKY passes and ordering | `chapter_19/*` |
| 20 | Register allocation | liveness, coloring, spill/coalesce | `chapter_20/int_only`, `chapter_20/all_types`, `chapter_20/helper_libs` |
