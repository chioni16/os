extern rust_start

global long_mode_start

section .text
bits 64
long_mode_start:
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    pop rdi         ; pass multiboot information pointer to kernel start
    call rust_start
    