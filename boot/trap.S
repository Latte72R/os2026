      .section .text
      .align 2
      .globl kernel_entry_trap

  kernel_entry_trap:
      # U-mode実行中:
      #   sp       = user_sp
      #   sscratch = kernel_stack_top
      #
      # S-mode実行中:
      #   sp       = kernel_sp
      #   sscratch = 0
      #
      # 入れ替えた後、spが0でなければU-modeから来たと判断する。
      csrrw sp, sscratch, sp
      bnez sp, .Ltrap_from_user

  .Ltrap_from_supervisor:
      # S-modeからのトラップでは、交換後の状態は、
      #
      #   sp       = 0
      #   sscratch = 元のkernel_sp
      #
      # なので、もう一度交換して元へ戻す。
      csrrw sp, sscratch, sp

      # 34 slots × 8 bytes = 272 bytes (16-byte aligned)
      #
      # slots 0..31: TrapFrame (slot 31 is sepc)
      # slot 32: U-mode由来かを表す内部フラグ
      # slot 33: alignment padding
      addi sp, sp, -34 * 8

      # 後で一時レジスタとしてt0を使うため、先に保存する。
      sd t0, 3 * 8(sp)

      # slot 32 = 0ならS-mode由来
      sd zero, 32 * 8(sp)

      # トラップ直前のkernel_spを保存する。
      addi t0, sp, 34 * 8
      sd t0, 30 * 8(sp)

      j .Lsave_registers

  .Ltrap_from_user:
      # U-modeからのトラップでは、交換後の状態は、
      #
      #   sp       = kernel_stack_top
      #   sscratch = 元のuser_sp
      #
      # となっている。
      addi sp, sp, -34 * 8

      # 後で一時レジスタとしてt0を使うため、先に保存する。
      sd t0, 3 * 8(sp)

      # slot 32 = 1ならU-mode由来
      li t0, 1
      sd t0, 32 * 8(sp)

      # sscratchに退避されている元のuser_spをTrapFrameへ保存する。
      csrr t0, sscratch
      sd t0, 30 * 8(sp)

      # これ以降はS-modeのカーネル内で処理する。
      # カーネル内でさらにトラップが起きても判定できるよう0にする。
      csrw sscratch, zero

  .Lsave_registers:
      csrr t0, sepc
      sd t0, 31 * 8(sp)

      sd ra,   0 * 8(sp)
      sd gp,   1 * 8(sp)
      sd tp,   2 * 8(sp)

      # t0は分岐前に保存済み
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

      # a0に&mut TrapFrameを渡す。
      mv a0, sp
      call handle_trap

  .Lrestore_registers:
      ld ra,   0 * 8(sp)
      ld gp,   1 * 8(sp)
      ld tp,   2 * 8(sp)

      # t0は復帰元判定の後で復元する。
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

      # 別プロセスのtrap処理をまたいでも元のPCへ戻れるようsepcを復元する。
      ld t0, 31 * 8(sp)
      csrw sepc, t0

      # slot 32でU-mode由来かを判定する。
      ld t0, 32 * 8(sp)
      bnez t0, .Lreturn_to_user

  .Lreturn_to_supervisor:
      # 元のt0を復元する。
      ld t0, 3 * 8(sp)

      # TrapFrameに保存した元のkernel_spを直接spへ戻す。
      ld sp, 30 * 8(sp)

      # S-mode実行中なのでsscratchは0のまま。
      sret

  .Lreturn_to_user:
      # TrapFrameに保存していたuser_spをsscratchへ入れる。
      ld t0, 30 * 8(sp)
      csrw sscratch, t0

      # 元のt0を復元する。
      ld t0, 3 * 8(sp)

      # TrapFrameを破棄する。
      # この時点のspはkernel_stack_topになる。
      addi sp, sp, 34 * 8

      # 入れ替えると、
      #
      #   sp       = user_sp
      #   sscratch = kernel_stack_top
      #
      # となる。
      csrrw sp, sscratch, sp
      sret
