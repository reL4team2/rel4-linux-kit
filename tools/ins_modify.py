#!/usr/bin/python3
import capstone
import lief
import os
import sys

if len(sys.argv) <= 2:
    print("Current DIR: " + os.getcwd())
    print("Usage: python3 modify.py <src> <dst>")
    exit(0)

src = sys.argv[1]
dst = sys.argv[2]
(path, filename) = os.path.split(src)
print("filename " + filename)
print("src " + src)
print("dst " + dst)

elf = lief.ELF.parse(src)

if elf is None:
    print("This is not a valid elf file")
    exit(-1)

text_section = elf.get_section(".text")
text_data = text_section.content.tolist()  # 获取 `.text` 段的二进制数据

md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_ARM)
md.disasm(bytes(text_data), text_section.virtual_address)

for ins in md.disasm(bytes(text_data), text_section.virtual_address):
    if ins.mnemonic == "svc":
        offset = ins.address - text_section.virtual_address
        text_data[offset : offset + 4] = (0xDEADBEEF).to_bytes(4, byteorder="little")

# 写回修改后的 .text 数据
text_section.content = text_data  # 关键点：重新赋值给 LIEF 结构

if os.path.isdir(dst):
    elf.write(os.path.join(dst, filename))
else:
    elf.write(dst)
# 保存修改后的 ELF
# elf.write(dst)
