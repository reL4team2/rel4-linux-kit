#!/usr/bin/python3
from capstone import *
import lief
import os
import sys

if len(sys.argv) <= 2:
    print("Current DIR: " + os.getcwd())
    print("Usage: python3 modify.py <src> <dst>")
    exit(0)

src = sys.argv[1]
dst = sys.argv[2]
print("src " + src)
print("dst " + dst)

elf = lief.ELF.parse(src)

text_section = elf.get_section(".text")
text_data = list(text_section.content)  # 获取 `.text` 段的二进制数据

md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
md.disasm(bytes(text_data), text_section.virtual_address)

for ins in md.disasm(bytes(text_data), text_section.virtual_address):
    if ins.mnemonic == "svc":
        offset = ins.address - text_section.virtual_address
        text_data[offset : offset + 4] = (0xDEADBEEF).to_bytes(4, byteorder="little")

# 写回修改后的 .text 数据
text_section.content = text_data  # 关键点：重新赋值给 LIEF 结构

# 保存修改后的 ELF
elf.write(dst)
