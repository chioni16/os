default rel

extern check_multiboot
extern check_cpuid
extern check_long_mode

extern set_up_page_tables
extern enable_paging
extern long_mode_start

global HIGHER_HALF

global start
global error

global p4_table
global p3_table
global p2_table
global low_p4_table
global low_p3_table
global low_p2_table
global HIGHER_HALF
global gdt64_pointer

global stack_bottom
global stack_top

;HIGHER_HALF: equ 0xC0000000
;low_stack_top: equ  stack_top - 0xC0000000
;low_stack_bottom: equ  stack_bottom - 0xC0000000
;low_p4_table: equ  p4_table - 0xC0000000
;low_p3_table: equ  p3_table - 0xC0000000
;low_p2_table: equ  p2_table - 0xC0000000
;low_gdt64_pointer: equ  gdt64_pointer - 0xC0000000
;low_gdt64_code: equ  gdt64.code - 0xC0000000
;low_long_mode_start: equ long_mode_start - 0xC0000000

HIGHER_HALF: equ 0xFFFF800000000000
low_stack_top: equ  stack_top - 0xFFFF800000000000
low_stack_bottom: equ  stack_bottom - 0xFFFF800000000000
low_p4_table: equ  p4_table - 0xFFFF800000000000
low_p3_table: equ  p3_table - 0xFFFF800000000000
low_p2_table: equ  p2_table - 0xFFFF800000000000
low_gdt64_pointer: equ  gdt64_pointer - 0xFFFF800000000000
low_gdt64_code: equ  gdt64.code - 0xFFFF800000000000
low_long_mode_start: equ long_mode_start - 0xFFFF800000000000



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

    ; experiment start

    ;; point rsp, rip to higher half addresses
    ;add esp, HIGHER_HALF
    ;mov eax, higher_half_addresses
    ;jmp eax
    ;higher_half_addresses:
    ;; clear identity mapping that we will not use anymore after moving to higher half addresses
    ;mov dword [low_p3_table], 0x0

    ;pop ebx
    ;jmp gdt64.code:long_mode_start

    ; experiment first part end

    jmp low_gdt64_code:low_long_mode_start

    ; experiment second part end

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
p2_table:
    resb 4096
stack_bottom:
    resb 64*1024*1024
stack_top:


section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
gdt64_pointer:
    dw $ - gdt64 - 1
    dq gdt64