    .globl main
main:
    pushq %rbp
    movq %rsp, %rbp
    subq $32, %rsp
    movq %rsp, %rdi
    movq $0, %rax
    call return_in_mem
    cmpq $1, 0(%rax)
    jne .Lfail
    cmpq $2, 8(%rax)
    jne .Lfail
    cmpq $3, 16(%rax)
    jne .Lfail
    movq $0, %rax
    movq %rbp, %rsp
    popq %rbp
    ret
.Lfail:
    movl $11, %edi
    call raise@PLT
    movq %rbp, %rsp
    popq %rbp
    ret
    .section .note.GNU-stack,"",@progbits
