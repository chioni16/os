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
    }
    .rodata : {
        *(.rodata)
    }
    .bss :
    {
        *(.bss)
    }
}