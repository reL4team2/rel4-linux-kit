#!/usr/bin/env python3
from os import path
import sys
import tomllib
from typing import List

# 一个简单的图解析算法：
#   当一个程序的入度（被依赖）大于 1 且有自己的资源，
#   那么这个程序就会被视为是一个单独的顶层模块（其他程序与其 IPC 通信）
#   如果一个模块与其为其他模块为 IPC 通信，那么会在编译时添加特定feature
#   例：test-demo 依赖了 uart-thread，如果需要使用 ipc 通信 ，那么在
#      编译 test-demo 时添加 uart-ipc，也许可以使用 cfg

FILE_DIR = path.dirname(path.realpath(__file__))


class Task:
    def __init__(
        self, name: str, file: str, mem: list, dma: list, deplist: list, cfglist: list
    ):
        self.name = name
        self.file = file
        self.mem = mem
        self.dma = dma
        self.deplist = deplist
        self.cfglist = cfglist
        self.deptask = []
        self.in_degree = 0

    def __repr__(self):
        ret = "Task {\n"
        ret += f"\tname = {self.name}\n"
        ret += f"\tfile = {self.file}\n"
        ret += f"\tmem = {self.get_mems()}\n"
        ret += f"\tdma = {self.get_dmas()}\n"
        ret += f"\tdeps = {self.deplist}\n"
        ret += "}"
        return ret

    # TODO: 检测环的存在，但是 cargo 要求不能存在环，其实也不需要检测
    def init(self):
        for task_name in self.deplist:
            self.deptask.append(tasks[task_name])

    def select(self):
        self.in_degree += 1
        if self.in_degree > 1:
            return
        for task in self.deptask:
            task.select()

    def get_mems(self):
        ret = []
        ret.extend(self.mem)
        for task in self.deptask:
            if task.in_degree > 1:
                continue
            ret.extend(task.get_mems())
        return ret

    def get_dmas(self):
        ret = []
        ret.extend(self.dma)
        for task in self.deptask:
            if task.in_degree > 1:
                continue
            ret.extend(task.get_dmas())
        return ret


# tasks: List[Task] = {}
tasks: dict[str, Task] = {}


def get_all_standalone_tasks():
    ret = []
    for task in tasks.values():
        # 小于等于 1 的不是没有使用就是只是依赖
        if task.in_degree <= 1:
            continue
        ret.append(task)
    return ret


def parse_config(config):
    for task in config["tasks"]:
        task_obj = Task(
            task["name"],
            task["file"],
            task.get("mem", []),
            task.get("dma", []),
            task.get("deps", []),
            task.get("cfg", []),
        )
        tasks[task["name"]] = task_obj
    for task in tasks.values():
        task.init()


def write_to_file(file):
    print(file)
    # 输出到 root-task 的配置文件
    output = "pub const TASK_FILES: &[KernelServices] = &[ \n"
    for task in get_all_standalone_tasks():
        output += "service! { \n"
        output += 'name: "%s", \n' % (task.name)
        output += 'file: "%s", \n' % (task.file)
        mem_list = [
            "(%s, %s, %s)" % (mem[0], mem[1], mem[2]) for mem in task.get_mems()
        ]
        output += "mem: &[%s],\n" % (",\n".join(mem_list))

        dma_list = ["(%s, %s)" % (dma[0], dma[1]) for dma in task.get_dmas()]
        output += "dma: &[%s],\n" % (",\n".join(dma_list))

        output += "},"

    output += "];"
    with open(file, "w") as f:
        f.write(output)
        f.close()

    # 输出到 Makefile 的配置文件
    configs = []
    for task in get_all_standalone_tasks():
        configs.extend(task.cfglist)
    configs = list(map(lambda x: "--cfg=" + x, configs))
    output = "RUSTFLAGS += " + " ".join(configs)

    mk_conf_file = path.join(FILE_DIR, "autoconfig.mk")
    with open(mk_conf_file, "w") as f:
        f.write(output)
        f.close()

def parse_selected(file):
    selected = tomllib.load(open(file, "rb"))
    selected = selected.get("module-selected", [])
    return selected

if __name__ == "__main__":
    file = open("apps.toml", "rb")
    data = tomllib.load(file)
    parse_config(data)
    if len(sys.argv) < 2:
        print("pass the config-select file, pls")
        exit(0)
    # targets = sys.argv[1:]
    select_file = sys.argv[1]
    targets = parse_selected(select_file)
    if len(targets) == 0:
        print("need at least one target to handle")
        exit(0)
    for selected in targets:
        tasks[selected].select()
        if tasks[selected].in_degree <= 1:
            tasks[selected].in_degree += 1
    print(get_all_standalone_tasks())
    write_to_file(path.join(FILE_DIR, "../root-task/src/autoconfig.rs"))

    # for task in data["tasks"]:
    #     print(task)
