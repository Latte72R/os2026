.option norvc
.section .text.init
.globl _start

_start:
    li a0, 0
    li a1, 0
    li t0, 0x80000000
    jr t0
