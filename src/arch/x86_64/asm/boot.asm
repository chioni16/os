MAGIC              equ 0xE85250D6
ARCH               equ 0x0
HEADER_LENGTH      equ header_end - header_start
CHECKSUM           equ -( MAGIC + ARCH + HEADER_LENGTH )

section .multiboot_header
header_start:
dd MAGIC
dd ARCH
dd HEADER_LENGTH
dd CHECKSUM
dw 0x0
dw 0x0
dq 0x8
header_end: