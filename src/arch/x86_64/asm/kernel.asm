default rel

extern check_multiboot
extern check_cpuid
extern check_long_mode
extern set_up_page_tables
extern enable_paging
extern long_mode_start

global start
global error
global p4_table
global p3_table
global low_p4_table
global low_p3_table
global gdt64
global gdt64_pointer

; for debugging purposes
global stack_bottom
global stack_top

; TODO can I get this from the linker script? extern doesn't seem to be working as intended
HIGHER_HALF: equ 0xFFFF800000000000
low_stack_top: equ  stack_top - HIGHER_HALF
low_stack_bottom: equ  stack_bottom - HIGHER_HALF
low_p4_table: equ  p4_table - HIGHER_HALF
low_p3_table: equ  p3_table - HIGHER_HALF
low_gdt64_pointer: equ  gdt64_pointer - HIGHER_HALF
low_long_mode_start: equ long_mode_start - HIGHER_HALF

section .text
bits 32
start:
    mov esp, low_stack_top ; stack set up
    push ebx ; push multiboot information pointer

    call check_multiboot
    call check_cpuid
    call check_long_mode

    call set_up_page_tables
    call enable_paging

    lgdt [low_gdt64_pointer]

    jmp gdt64.code:low_long_mode_start ; enter long mode

error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .bss
p4_table:
    resb 4096
p3_table:
    resb 4096
stack_bottom:
    resb 64*1024*1024
stack_top:

section .rodata
; some of the flags set below are ignored in the 64 bit mode
; base and length of descriptors are not set as they are ignored in 64 bit mode
align 8
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64
    dq (1 << 40) | (1 << 41) | (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment (accessed, readable, executable, non-system segment, present, long mode)
.data: equ $ - gdt64
    dq (1 << 40) | (1 << 41) | (1<<44) | (1<<47); data segment (accessed, writable, non-system segment, present)
.tss: equ $ - gdt64
    ; leave enough space for TSS entry (16 bytes)
    ; fill in the correct value in rust code
    dq 0
    dq 0
.udata: equ $ - gdt64
    dq (1 << 40) | (1 << 41) | (1<<44) | (1<<47) | (3 << 45) ; user data segment (accessed, writable, non-system segment, present, user mode)
.ucode: equ $ - gdt64
    dq (1 << 40) | (1 << 41) | (1<<43) | (1<<44) | (1<<47) | (1<<53) | (3 << 45); user code segment (accessed, readable, executable, non-system segment, present, long mode, user mode)
gdt64_pointer:
    dw $ - gdt64 - 1
    dq gdt64
