default rel

extern rust_start
extern p3_table
extern p4_table
extern gdt64_pointer

global long_mode_start

section .text
bits 64
long_mode_start:
    ; experiment start

    ; point rsp, rip to higher half addresses
    mov rax, 0xFFFF800000000000
    add rsp, rax
    mov rax, higher_half_addresses
    jmp rax
    higher_half_addresses:
    ; clear identity mapping that we will not use anymore after moving to higher half addresses
    ; mov dword [p3_table], 0x0
    lgdt [gdt64_pointer]
    mov dword [p4_table], 0x0

    pop rbx

    ; experiment end

    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    mov edi, ebx         ; pass multiboot information pointer to kernel start
    call rust_start
