    .section .text
    .align 2
    .globl kernel_entry_trap

kernel_entry_trap:
    addi sp, sp, -16 * 8

    sd ra,  0 * 8(sp)
    sd t0,  1 * 8(sp)
    sd t1,  2 * 8(sp)
    sd t2,  3 * 8(sp)
    sd a0,  4 * 8(sp)
    sd a1,  5 * 8(sp)
    sd a2,  6 * 8(sp)
    sd a3,  7 * 8(sp)
    sd a4,  8 * 8(sp)
    sd a5,  9 * 8(sp)
    sd a6, 10 * 8(sp)
    sd a7, 11 * 8(sp)
    sd t3, 12 * 8(sp)
    sd t4, 13 * 8(sp)
    sd t5, 14 * 8(sp)
    sd t6, 15 * 8(sp)

    call handle_trap

    ld ra,  0 * 8(sp)
    ld t0,  1 * 8(sp)
    ld t1,  2 * 8(sp)
    ld t2,  3 * 8(sp)
    ld a0,  4 * 8(sp)
    ld a1,  5 * 8(sp)
    ld a2,  6 * 8(sp)
    ld a3,  7 * 8(sp)
    ld a4,  8 * 8(sp)
    ld a5,  9 * 8(sp)
    ld a6, 10 * 8(sp)
    ld a7, 11 * 8(sp)
    ld t3, 12 * 8(sp)
    ld t4, 13 * 8(sp)
    ld t5, 14 * 8(sp)
    ld t6, 15 * 8(sp)

    addi sp, sp, 16 * 8
    sret
