default rel

extern low_p4_table
extern low_p3_table

global set_up_page_tables
global enable_paging

section .text
bits 32
set_up_page_tables:
    ; map first P4 entry to P3 table
    mov eax, low_p3_table
    or eax, 0b11 ; present + writable
    mov [low_p4_table], eax

    ; map higher address space as well
    mov [low_p4_table + 0x100 * 8], eax

    ; map 4GiB of physical memory (if this value is changed, remember change the value in to_virt method of PhysicalAddress as well)
    mov eax, 0
    or eax, 0b10000011 ; present + writable + huge (1 GiB)
    mov [low_p3_table + 0], eax
    add eax, 0x40000000
    mov [low_p3_table + 8], eax
    add eax, 0x40000000
    mov [low_p3_table + 16], eax
    add eax, 0x40000000
    mov [low_p3_table + 24], eax

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, low_p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret
