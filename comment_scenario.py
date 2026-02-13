#!/usr/bin/env python3
import sys
import re

path = sys.argv[1]
with open(path, 'r') as f:
    lines = f.readlines()

in_block = False
new_lines = []
for line in lines:
    if '// Ensure \'core\' scenario is installed on startup' in line:
        in_block = True
        new_lines.append('// Ensure \'core\' scenario is installed on startup - DISABLED FOR MCP DEBUG\n')
        continue
    if in_block and line.strip().startswith('Err(e) => eprintln!("Failed to load core scenario:"'):
        new_lines.append('// ' + line)
        in_block = False
        continue
    if in_block:
        new_lines.append('// ' + line)
    else:
        new_lines.append(line)

with open(path, 'w') as f:
    f.writelines(new_lines)
print("Commented out scenario installation block.")