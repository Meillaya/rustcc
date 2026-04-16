# Test Map

## Test strategy summary

- chapters 1–18 mostly mix `valid/` and `invalid_*` directories
- chapter 13 and chapter 20 include helper-library-sensitive tests
- chapter 19 uses optimization-family directories instead of only `valid/` and `invalid_*`
- chapter 20 is split by language richness (`int_only`, `all_types`) and by coalescing expectations

## Chapter-to-test map

| Chapter | Primary tests | Recommended command |
|---|---|---|
| 1 | `chapter_1/valid`, `invalid_lex`, `invalid_parse` | `./tests/test_compiler ./target/release/rustcc --chapter 1 --latest-only --expected-error-codes 1 2` |
| 2 | `chapter_2/valid`, `invalid_parse` | `./tests/test_compiler ./target/release/rustcc --chapter 2 --latest-only` |
| 3 | `chapter_3/valid`, `invalid_parse` | `./tests/test_compiler ./target/release/rustcc --chapter 3 --latest-only --bitwise` |
| 4 | `chapter_4/valid`, `invalid_parse` | `./tests/test_compiler ./target/release/rustcc --chapter 4 --latest-only --bitwise` |
| 5 | `chapter_5/valid`, `invalid_parse`, `invalid_semantics` | `./tests/test_compiler ./target/release/rustcc --chapter 5 --latest-only --bitwise --compound --increment` |
| 6 | `chapter_6/valid`, `invalid_lex`, `invalid_parse`, `invalid_semantics` | `./tests/test_compiler ./target/release/rustcc --chapter 6 --latest-only --bitwise --compound --increment --goto` |
| 7 | `chapter_7/valid`, `invalid_parse`, `invalid_semantics` | `./tests/test_compiler ./target/release/rustcc --chapter 7 --latest-only --compound --goto` |
| 8 | `chapter_8/valid`, `invalid_parse`, `invalid_semantics` | `./tests/test_compiler ./target/release/rustcc --chapter 8 --latest-only --compound --increment --goto --switch` |
| 9 | `chapter_9/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 9 --latest-only --bitwise --compound --increment --goto --switch` |
| 10 | `chapter_10/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 10 --latest-only` |
| 11 | `chapter_11/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 11 --latest-only` |
| 12 | `chapter_12/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 12 --latest-only` |
| 13 | `chapter_13/valid`, `helper_libs`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 13 --latest-only --nan` |
| 14 | `chapter_14/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 14 --latest-only` |
| 15 | `chapter_15/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 15 --latest-only` |
| 16 | `chapter_16/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 16 --latest-only` |
| 17 | `chapter_17/valid`, `invalid_*` | `./tests/test_compiler ./target/release/rustcc --chapter 17 --latest-only` |
| 18 | `chapter_18/valid`, `invalid_*`, `valid/extra_credit` | `./tests/test_compiler ./target/release/rustcc --chapter 18 --latest-only --union` |
| 19 | `chapter_19/constant_folding`, `unreachable_code_elimination`, `copy_propagation`, `dead_store_elimination`, `whole_pipeline` | `./tests/test_compiler ./target/release/rustcc --chapter 19 --latest-only --eliminate-dead-stores` |
| 20 | `chapter_20/int_only`, `chapter_20/all_types`, `helper_libs` | `./tests/test_compiler ./target/release/rustcc --chapter 20 --latest-only --no-coalescing` |

## Notes

- use `--skip-invalid` only as a temporary backend-development aid
- rerun without `--skip-invalid` before calling a chapter complete
- chapter 19 and 20 should usually be tested at multiple stage stops, not only at `run`
