ENTRY(start)

HIGHER_HALF = 0xFFFF800000000000;
/* HIGHER_HALF = 0xC0000000; */

SECTIONS {
    . = 1M;
    . += HIGHER_HALF;

    .boot ALIGN (4K) : AT (ADDR (.boot) - HIGHER_HALF)
    {
        *(.multiboot_header)
    }

    .text ALIGN (4K) : AT (ADDR (.text) - HIGHER_HALF)
    {
        *(.text .text.*)
    }
    .rodata ALIGN (4K) : AT (ADDR (.rodata) - HIGHER_HALF)
    {
        *(.rodata .rodata.*)
    }
    .data.rel.ro ALIGN (4K) : AT (ADDR (.data.rel.ro) - HIGHER_HALF)
    {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    }
    .bss ALIGN (4K) : AT (ADDR (.bss) - HIGHER_HALF)
    {
        *(.bss .bss.*)
    }
}