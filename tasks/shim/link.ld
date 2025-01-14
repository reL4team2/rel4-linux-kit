OUTPUT_ARCH(aarch64)
ENTRY(_start)

BASE_ADDRESS = 0x100001000;

SECTIONS
{
    /* Load the kernel at this address: "." means the current address */
    . = BASE_ADDRESS;

    .text ALIGN(4K): {
        .text._start = .;
        *(.text .text.*)
    }

    .rodata ALIGN(4K): {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K): {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .tbss : {
        *(.tbss .tbss.*)
    }

    .gcc : {
        *(.gcc .gcc.*)
    }

    .bss ALIGN(4K): {
        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        _ebss = .;
    }

    PROVIDE(_end = .);
    /DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}
