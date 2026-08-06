OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  . = 0x80200000;
  __kernel_base = .;

  .text : ALIGN(16) {
    *(.text.entry)
    *(.text .text.*)
  }

  .rodata : ALIGN(16) {
    *(.rodata .rodata.*)
  }

  .data : ALIGN(16) {
    *(.data .data.*)
  }

  .bss : ALIGN(16) {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  }

  . = ALIGN(16);
  . += 64K;
  __stack_top = .;

  . = ALIGN(4096);
  __heap_start = .;
  . += 8M;
  __heap_end = .;
}
