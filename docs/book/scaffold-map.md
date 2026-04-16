# Scaffold Map

This map explains what the in-`src/` skeleton files are meant to represent while the real implementation is still chapter-gated.

| Chapter | Scaffold focus | Suggested placeholder files |
|---|---|---|
| 1 | driver / minimal frontend / return-only backend | `lex/mod.rs`, `parse/mod.rs`, `ast/mod.rs`, `driver.rs`, `compiler.rs`, `toolchain.rs` |
| 2 | unary expressions | `ast/expr.rs`, `ast/operator.rs`, `parse/precedence.rs` |
| 3 | precedence and binary expressions | `parse/precedence.rs`, `ast/operator.rs`, `codegen/lower.rs` |
| 4 | boolean control flow | `ast/operator.rs`, `ir/control_flow.rs`, `codegen/lower.rs` |
| 5 | locals and assignments | `semantics/symbols.rs`, `semantics/validate.rs`, `codegen/frame.rs` |
| 6 | if / ternary / labels | `ast/stmt.rs`, `ir/control_flow.rs`, `codegen/lower.rs` |
| 7 | block scopes | `semantics/names.rs`, `semantics/symbols.rs`, `ast/stmt.rs` |
| 8 | loops | `ast/stmt.rs`, `ir/control_flow.rs`, `util/labels.rs` |
| 9 | functions and calls | `ast/item.rs`, `semantics/types.rs`, `codegen/abi.rs` |
| 10 | globals and linkage | `ast/decl.rs`, `semantics/names.rs`, `codegen/emit.rs` |
| 11 | long integers | `ast/ty.rs`, `semantics/types.rs`, `codegen/lower.rs` |
| 12 | unsigned integers | `ast/ty.rs`, `semantics/types.rs`, `codegen/lower.rs` |
| 13 | floating point | `ast/ty.rs`, `codegen/abi.rs`, `codegen/lower.rs` |
| 14 | pointers | `ast/ty.rs`, `ast/operator.rs`, `semantics/types.rs` |
| 15 | arrays and pointer arithmetic | `ast/ty.rs`, `semantics/layout.rs`, `codegen/frame.rs` |
| 16 | chars and strings | `lex/token.rs`, `ast/expr.rs`, `codegen/emit.rs` |
| 17 | `sizeof`, `void`, heap support | `semantics/layout.rs`, `semantics/validate.rs`, `ast/ty.rs` |
| 18 | structs / unions | `ast/decl.rs`, `semantics/layout.rs`, `codegen/abi.rs` |
| 19 | optimization passes | `ir/opt.rs`, `ir/tacky.rs`, `ir/control_flow.rs` |
| 20 | register allocation | `codegen/register_allocator.rs`, `ir/control_flow.rs`, `ir/temp.rs` |

## Global scaffold files

- `src/main.rs` — root module wiring for the skeleton
- `src/lex/mod.rs` — lexer subtree root
- `src/parse/mod.rs` — parser subtree root
- `src/semantics/mod.rs` — semantic-analysis subtree root
- `src/ir/mod.rs` — IR/TACKY subtree root
- `src/codegen/mod.rs` — backend subtree root
