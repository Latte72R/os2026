OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS {
    . = 0x80700000;

    .text : ALIGN(4096) {
        KEEP(*(.text.entry))
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4096) {
        *(.data .data.*)
    }

    .bss : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    . = ALIGN(4096);
    __user_end = .;

    ASSERT(__user_end <= 0x80800000, "user image exceeds reserved RAM")
}
