default rel

extern rust_start
extern p4_table
extern gdt64_pointer

global long_mode_start

section .text
bits 64
long_mode_start:
    ; point rsp, rip to higher half addresses
    mov rax, 0xFFFF800000000000
    add rsp, rax
    mov rax, higher_half_addresses
    jmp rax
    
    higher_half_addresses:
    
    ; clear identity mapping that we will not use anymore after moving to higher half addresses
    mov dword [p4_table], 0x0
    
    ; change gdtr to make it point to the higher address space
    lgdt [gdt64_pointer]

    ; get the multiboot info from bootloader
    pop rbx

    ; set segment regs
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    
    ; pass multiboot information pointer to kernel start
    mov edi, ebx         

    ; call rust code
    call rust_start
