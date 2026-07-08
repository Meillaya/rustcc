# Helper for push_arg_on_page_boundary.c.
# Place `zed` as the last byte of a page on Linux so an incorrect
# 8-byte stack push from memory crosses into the next unmapped page.
    .globl zed
    .bss
    .balign 4096
    .skip 4095
zed:
    .zero 1
