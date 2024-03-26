global ap_start

section .text.ap
bits 16
ap_start:
    mov al, 1
    out 0x3f8, al
.loop:
    hlt
    jmp .loop