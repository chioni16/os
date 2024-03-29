extern p4_table
extern p3_table
extern p2_table


global set_up_page_tables
global enable_paging

section .text
bits 32
set_up_page_tables:
    ; map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11 ; present + writable
    mov [p4_table], eax

    ; 1 GiB huge page identical mapping
    mov ecx, 0         ; counter variable
.map_p3_table:
    mov eax, 0x40000000           ; 1GB
    mul ecx                       ; start address of ecx-th page
    or eax, 0b10000011            ; present + writable + huge
    mov [p3_table + ecx * 8], eax ; map ecx-th entry

    inc ecx            ; increase counter
    cmp ecx, 4
    jne .map_p3_table  ; else map the next entry

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
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
