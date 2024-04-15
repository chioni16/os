#!/usr/bin/python3

def page_table_indices(virtual_addr):
    off = virtual_addr & 0xfff
    p1 = (virtual_addr >> 12) & 0o777
    p2 = (virtual_addr >> (12 + 9)) & 0o777
    p3 = (virtual_addr >> (12 + 9 + 9)) & 0o777
    p4 = (virtual_addr >> (12 + 9 + 9 + 9)) & 0o777
    print(f'p4: {hex(p4)}')
    print(f'p3: {hex(p3)}')
    print(f'p2: {hex(p2)}')
    print(f'p1: {hex(p1)}')
    print(f'off: {hex(off)}')
