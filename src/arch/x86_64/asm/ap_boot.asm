default rel

global ap_start

section .text.ap
bits 16
ap_start:
    mov al, 0x7a
    mov dx, 0x3f8
    out dx, al
.loop:
    ; hlt
    jmp .loop