OUTPUT_FORMAT(elf64-littleaarch64)
ENTRY(_start)
SECTIONS {
    . = 0x80000;
    .text   : { *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data   : { *(.data .data.*) }
    .bss    : { *(.bss .bss.*) *(COMMON) }
    /DISCARD/ : { *(.eh_frame) *(.note*) }
}
