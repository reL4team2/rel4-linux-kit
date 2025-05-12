import os, sys
import tomllib
from typing import List

# 一个简单的图解析算法：
#   当一个程序的入度（被依赖）大于 1 且有自己的资源，
#   那么这个程序就会被视为是一个单独的顶层模块（其他程序与其 IPC 通信）
#   如果一个模块与其为其他模块为 IPC 通信，那么会在编译时添加特定feature
#   例：test-demo 依赖了 uart-thread，如果需要使用 ipc 通信 ，那么在
#      编译 test-demo 时添加 uart-ipc，也许可以使用 cfg


class Task:
    def __init__(self, name: str, file: str, mem: list, deplist: list):
        self.name = name
        self.file = file
        self.mem = mem
        self.deplist = deplist
        self.deptask = []
        self.in_degree = 0

    def __repr__(self):
        ret = "Task {\n"
        ret += f"\tname = {self.name}\n"
        ret += f"\tfile = {self.file}\n"
        ret += f"\tmem = {self.mem}\n"
        ret += f"\tdeps = {self.deplist}\n"
        ret += "}"
        return ret

    # TODO: 检测环的存在，但是 cargo 要求不能存在环，其实也不需要检测
    def init(self):
        for task_name in self.deplist:
            self.deptask.append(tasks[task_name])

    def select(self):
        self.in_degree += 1
        for task in self.deptask:
            task.select()

    # todo: 根据依赖关系和入度出度，判断是否
    def all_mems(self):
        print(f"task: {self.name}: ")
        print(self.mem)
        for task in self.deptask:
            if task.in_degree > 1:
                continue
            print(task.mem)
        print()


tasks: List[Task] = {}


def get_all_standalone_tasks():
    ret = []
    for task in tasks.values():
        print(f"{task.name}: {task.in_degree}")
        # 小于等于 1 的不是没有使用就是只是依赖
        if task.in_degree <= 1:
            continue
        ret.append(task)
    return ret


def parse_config(config):
    for task in config["tasks"]:
        task_obj = Task(
            task["name"], task["file"], task.get("mem", []), task.get("deps", [])
        )
        tasks[task["name"]] = task_obj
    for task in tasks.values():
        task.init()
        task.all_mems()


if __name__ == "__main__":
    file = open("apps.toml", "rb")
    data = tomllib.load(file)
    parse_config(data)
    print(tasks)
    for selected in ["test-demo"]:
        tasks[selected].in_degree += 1
        tasks[selected].select()
    print(get_all_standalone_tasks())
    # for task in data["tasks"]:
    #     print(task)
