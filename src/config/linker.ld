ENTRY(start)

SECTIONS {
    .text.ap :
    {
        . = ALIGN(4096);
        _ap_start_location = .;
        *(.text.ap)
    }
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