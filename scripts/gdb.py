python while gdb.parse_and_eval('$rip') < 0x100000: gdb.execute('ni')
python gdb.execute('monitor xp/1xb 0x4212001')
