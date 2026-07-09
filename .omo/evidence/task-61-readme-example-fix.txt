Task61 README example path fix evidence
=======================================
Date: 2026-07-09T06:19:54-04:00
Repo: /home/mei/projects/rustcc
HEAD: 149b45f
Fix: added examples/hello.c so README Common examples use an existing source file.

$ git diff -- tests
(exit 0)

$ cargo fmt --all -- --check
(exit 0)

$ cargo build --release
    Finished `release` profile [optimized] target(s) in 0.05s
(exit 0)

$ cargo test --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running unittests src/lib.rs (target/release/deps/rustcc-41b78a55704c0e27)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/release/deps/rustcc-b48f2e14c29f3b0e)

running 10 tests
test compiler::tests::rejects_bad_lexeme ... ok
test compiler::tests::compiles_constant_return ... ok
test compiler::tests::compiles_expression_precedence ... ok
test compiler::tests::handles_locals_and_assignment ... ok
test compiler::tests::reaches_validate_through_pass_through_resolve ... ok
test driver::tests::derives_all_output_paths ... ok
test driver::tests::parses_default_run_stage ... ok
test driver::tests::parses_artifact_and_feature_flags ... ok
test compiler::tests::parses_sizeof_expression_without_evaluating_it ... ok
test driver::tests::parses_stage_flags_as_stdout_only ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests rustcc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(exit 0)

$ cargo clippy --release -- -W clippy::all
    Finished `release` profile [optimized] target(s) in 0.03s
(exit 0)

README exact example commands
-----------------------------

$ target/release/rustcc examples/hello.c
(exit 0)

$ test -x examples/hello
(exit 0)

$ target/release/rustcc -S examples/hello.c
(exit 0)

$ test -f examples/hello.s
(exit 0)

$ target/release/rustcc -c examples/hello.c
(exit 0)

$ test -f examples/hello.o
(exit 0)

$ target/release/rustcc --lex examples/hello.c
Int int
Identifier main
OpenParen (
Void void
CloseParen )
OpenBrace {
Return return
Constant 0
Semicolon ;
CloseBrace }
Eof
(exit 0)

$ target/release/rustcc --parse examples/hello.c
Program {
    top_level_items: [
        Function(
            Function {
                name: "main",
                ret_ty: Int,
                params: [],
                body: Some(
                    [
                        Statement(
                            Return(
                                Some(
                                    Constant(
                                        0,
                                    ),
                                ),
                            ),
                        ),
                    ],
                ),
                storage: Auto,
            },
        ),
    ],
}
(exit 0)

$ target/release/rustcc --validate examples/hello.c
validated: TypedProgram {
    program: Program {
        top_level_items: [
            Function(
                Function {
                    name: "main",
                    ret_ty: Int,
                    params: [],
                    body: Some(
                        [
                            Statement(
                                Return(
                                    Some(
                                        Constant(
                                            0,
                                        ),
                                    ),
                                ),
                            ),
                        ],
                    ),
                    storage: Auto,
                },
            ),
        ],
    },
}
(exit 0)

$ target/release/rustcc --tacky examples/hello.c
TackyProgram {
    functions: [
        TackyFunction {
            name: "main",
            global: true,
            params: [],
            body: [
                Return(
                    Constant(
                        0,
                    ),
                ),
            ],
            type_env: {},
            ast_type_env: {},
            return_type: Int,
        },
    ],
    static_variables: [],
    static_constants: [],
    function_param_types: {
        "main": [],
    },
    function_return_types: {
        "main": Int,
    },
}
(exit 0)

$ target/release/rustcc --codegen examples/hello.c
.text
.globl main
main:
    pushq %rbp
    movq %rsp, %rbp
    movl $0, %eax
    movq %rbp, %rsp
    popq %rbp
    ret

(exit 0)

$ rm -f examples/hello examples/hello.s examples/hello.o
(exit 0)

$ git diff --check
(exit 0)
