ENTRY(start)

SECTIONS {
    . = 1M;
    .boot :
    {
        *(.multiboot_header)
    }
    .text :
    {
        *(.text)
        *(.text.ap)
    }
    .rodata : {
        *(.rodata)
    }
    .bss :
    {
        *(.bss)
    }
}