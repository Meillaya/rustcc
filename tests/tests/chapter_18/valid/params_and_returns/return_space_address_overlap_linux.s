    .globl globvar
    .bss
    .balign 8
globvar:
    .zero 24
    .text
    .globl overlap_with_globvar
overlap_with_globvar:
    pushq %rbp
    movq %rsp, %rbp
    leaq globvar(%rip), %rax
    cmpq %rax, %rdi
    je .Lfail_glob
    movq $400, 0(%rdi)
    movq $500, 8(%rdi)
    movq $600, 16(%rdi)
    movq %rdi, %rax
    popq %rbp
    ret
.Lfail_glob:
    movl $11, %edi
    call raise@PLT
    movq %rdi, %rax
    popq %rbp
    ret
    .globl overlap_with_pointer
overlap_with_pointer:
    pushq %rbp
    movq %rsp, %rbp
    cmpq %rsi, %rdi
    je .Lfail_ptr
    movq 0(%rsi), %rax
    addq %rax, %rax
    movq %rax, 0(%rdi)
    movq 8(%rsi), %rax
    addq %rax, %rax
    movq %rax, 8(%rdi)
    movq 16(%rsi), %rax
    addq %rax, %rax
    movq %rax, 16(%rdi)
    movq %rdi, %rax
    popq %rbp
    ret
.Lfail_ptr:
    movl $11, %edi
    call raise@PLT
    movq %rdi, %rax
    popq %rbp
    ret
    .section .note.GNU-stack,"",@progbits
