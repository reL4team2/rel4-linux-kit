SECTIONS
{
    . = 0x40080000;       /* 程序从物理地址 0x80000 开始 */
    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss : { *(.bss*) }
}

