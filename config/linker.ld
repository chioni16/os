ENTRY(start)

HIGHER_HALF = 0xFFFF800000000000;

/* 
* TODO:
* WARNING: This may have to be modified when the kernel page table uses the segment permissions.
* At the moment, everything works as all the pages are mapped with RWX permissions 
*
*/

PHDRS {
    boot    PT_LOAD    FLAGS((1 << 2)) ;            /* Read only */
    ap_text PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    text    PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    rodata  PT_LOAD    FLAGS((1 << 2)) ;            /* Read only */
    data    PT_LOAD    FLAGS((1 << 1) | (1 << 2)) ; /* Write + Read */
    dynamic PT_DYNAMIC FLAGS((1 << 1) | (1 << 2)) ; /* Dynamic PHDR for relocations */
}

SECTIONS {
    . = HIGHER_HALF;
    
    .text.ap ALIGN (4K) : AT (ADDR (.text.ap) - HIGHER_HALF)
    {
        _ap_start_location = .;
        *(.text.ap)
    } :ap_text

    . = HIGHER_HALF + 1M;

    .boot ALIGN (4K) : AT (ADDR (.boot) - HIGHER_HALF)
    {
        *(.multiboot_header)
    } :boot

    .text ALIGN (4K) : AT (ADDR (.text) - HIGHER_HALF)
    {
        *(.text .text.*)
    } :text

    .rodata ALIGN (4K) : AT (ADDR (.rodata) - HIGHER_HALF)
    {
        *(.rodata .rodata.*)
    } :rodata

    .data.rel.ro ALIGN (4K) : AT (ADDR (.data.rel.ro) - HIGHER_HALF)
    {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    } :rodata

    .dynamic ALIGN (4K) : AT (ADDR (.dynamic) - HIGHER_HALF)
    {
        *(.dynamic .dynamic.*)
    } :dynamic

    .bss ALIGN (4K) : AT (ADDR (.bss) - HIGHER_HALF)
    {
        *(.bss .bss.*)
    } :data
}