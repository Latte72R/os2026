.section .text.entry
.globl _start

_start:
    call main

    li a7, 3
    ecall

1:  j 1b
