#!/usr/bin/python3

import sys
import subprocess

addrs = sys.argv[1:]
print(addrs)

for addr in addrs:
    print(addr)
    result = subprocess.check_output(['addr2line', '-e', 'target/bin/kernel', addr], text=True) 
    print(result)