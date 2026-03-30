    .section .text.entry
    .globl _start

_start:
    la sp, __stack_top
    call rust_main

1:
    wfi
    j 1b
