    .section .text
    .align 2
    .globl switch_context_asm

switch_context_asm:
    # 16 slots × 8 bytes = 128 bytes
    addi sp, sp, -16 * 8

    sd ra,   0 * 8(sp)
    sd s0,  1 * 8(sp)
    sd s1,  2 * 8(sp)
    sd s2,  3 * 8(sp)
    sd s3,  4 * 8(sp)
    sd s4,  5 * 8(sp)
    sd s5,  6 * 8(sp)
    sd s6,  7 * 8(sp)
    sd s7,  8 * 8(sp)
    sd s8,  9 * 8(sp)
    sd s9,  10 * 8(sp)
    sd s10, 11 * 8(sp)
    sd s11, 12 * 8(sp)

    sd sp, 0(a0)
    ld sp, 0(a1)

    ld ra,   0 * 8(sp)
    ld s0,  1 * 8(sp)
    ld s1,  2 * 8(sp)
    ld s2,  3 * 8(sp)
    ld s3,  4 * 8(sp)
    ld s4,  5 * 8(sp)
    ld s5,  6 * 8(sp)
    ld s6,  7 * 8(sp)
    ld s7,  8 * 8(sp)
    ld s8,  9 * 8(sp)
    ld s9,  10 * 8(sp)
    ld s10, 11 * 8(sp)
    ld s11, 12 * 8(sp)
    
    addi sp, sp, 16 * 8
    ret
