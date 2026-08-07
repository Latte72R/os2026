    .section .text
    .align 2
    .globl kernel_entry_trap

kernel_entry_trap:
    # 32 slots × 8 bytes = 256 bytes
    addi sp, sp, -32 * 8

    sd ra,   0 * 8(sp)
    sd gp,   1 * 8(sp)
    sd tp,   2 * 8(sp)

    sd t0,   3 * 8(sp)
    sd t1,   4 * 8(sp)
    sd t2,   5 * 8(sp)
    sd t3,   6 * 8(sp)
    sd t4,   7 * 8(sp)
    sd t5,   8 * 8(sp)
    sd t6,   9 * 8(sp)

    sd a0,  10 * 8(sp)
    sd a1,  11 * 8(sp)
    sd a2,  12 * 8(sp)
    sd a3,  13 * 8(sp)
    sd a4,  14 * 8(sp)
    sd a5,  15 * 8(sp)
    sd a6,  16 * 8(sp)
    sd a7,  17 * 8(sp)

    sd s0,  18 * 8(sp)
    sd s1,  19 * 8(sp)
    sd s2,  20 * 8(sp)
    sd s3,  21 * 8(sp)
    sd s4,  22 * 8(sp)
    sd s5,  23 * 8(sp)
    sd s6,  24 * 8(sp)
    sd s7,  25 * 8(sp)
    sd s8,  26 * 8(sp)
    sd s9,  27 * 8(sp)
    sd s10, 28 * 8(sp)
    sd s11, 29 * 8(sp)

    # 元のspは、現在のspより256バイト上
    addi t0, sp, 32 * 8
    sd t0, 30 * 8(sp)

    # 第1引数a0として&mut TrapFrameを渡す
    mv a0, sp
    call handle_trap

    ld ra,   0 * 8(sp)
    ld gp,   1 * 8(sp)
    ld tp,   2 * 8(sp)

    ld t0,   3 * 8(sp)
    ld t1,   4 * 8(sp)
    ld t2,   5 * 8(sp)
    ld t3,   6 * 8(sp)
    ld t4,   7 * 8(sp)
    ld t5,   8 * 8(sp)
    ld t6,   9 * 8(sp)

    ld a0,  10 * 8(sp)
    ld a1,  11 * 8(sp)
    ld a2,  12 * 8(sp)
    ld a3,  13 * 8(sp)
    ld a4,  14 * 8(sp)
    ld a5,  15 * 8(sp)
    ld a6,  16 * 8(sp)
    ld a7,  17 * 8(sp)

    ld s0,  18 * 8(sp)
    ld s1,  19 * 8(sp)
    ld s2,  20 * 8(sp)
    ld s3,  21 * 8(sp)
    ld s4,  22 * 8(sp)
    ld s5,  23 * 8(sp)
    ld s6,  24 * 8(sp)
    ld s7,  25 * 8(sp)
    ld s8,  26 * 8(sp)
    ld s9,  27 * 8(sp)
    ld s10, 28 * 8(sp)
    ld s11, 29 * 8(sp)

    ld sp, 30 * 8(sp)
    sret
