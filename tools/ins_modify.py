#!/usr/bin/python3
import capstone
import lief
import os
import sys


# 修改指令，如果修改成功返回 true, 修改失败返回 false
def modify_ins(src, dst) -> bool:
    elf = lief.ELF.parse(src)
    if elf is None:
        return False

    (_, filename) = os.path.split(src)
    # 读取程序的 txt 段
    text_section = elf.get_section(".text")
    text_data = text_section.content.tolist()  # 获取 `.text` 段的二进制数据
    # 设置 capstone 反汇编的类型
    md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_ARM)
    # 遍历 svc 指令并修改为 0xDEADBEEF
    for ins in md.disasm(bytes(text_data), text_section.virtual_address):
        if ins.mnemonic == "svc":
            offset = ins.address - text_section.virtual_address
            text_data[offset : offset + 4] = (0xDEADBEEF).to_bytes(
                4, byteorder="little"
            )

    # 写回修改后的 .text 数据
    text_section.content = text_data  # 关键点：重新赋值给 LIEF 结构

    if os.path.isdir(dst):
        elf.write(os.path.join(dst, filename))
    else:
        elf.write(dst)
    return True


if __name__ == "__main__":
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
    if not modify_ins(src, dst):
        print("This is not a valid elf file: " + src)
