    .section .text
    .align 2
    .globl enter_user_asm

enter_user_asm:
    # a0 = user entry, a1 = user stack top, a2 = initial argument,
    # a3 = kernel stack top used by trap entry.
    csrw sepc, a0
    csrw sscratch, a3

    # Return to U-mode (SPP=0) with interrupts enabled after sret (SPIE=1).
    li t0, (1 << 8)
    csrc sstatus, t0
    li t0, (1 << 5)
    csrs sstatus, t0

    mv sp, a1
    mv a0, a2
    sret
