#!/usr/bin/env python3

import os, sys
import shutil
import ins_modify


# 处理文件，如果修改指令失败，直接复制文件到目标目录
def handle_file(src, dst, file):
    src = os.path.join(src, file)
    if not ins_modify.modify_ins(src, dst):
        shutil.copy(src, dst)
    print("file %s -> %s" % (src, os.path.join(dst, file)))


if __name__ == "__main__":
    if len(sys.argv) <= 2:
        print("Current DIR: " + os.getcwd())
        print("Usage: python3 modify.py <src_dir> <dst_dir>")
        exit(0)
    src = sys.argv[1]
    if not os.path.exists(src):
        print("Path is not exists: " + src)
        exit(0)
    dst = sys.argv[2]

    for curr_dir, dirs, files in os.walk(src):
        rela_path = os.path.relpath(curr_dir, src)
        curr_dst = os.path.join(dst, rela_path)
        # 处理子文件夹，如果文件不存在，就创建一个
        for dir in dirs:
            new_dir = os.path.join(curr_dst, dir)
            if not os.path.exists(new_dir):
                os.mkdir(new_dir)
        # 处理文件
        for file in files:
            handle_file(curr_dir, curr_dst, file)
