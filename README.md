# rel4-linux-kit

> rel4 linux kit 提供一套工具和服务以便于 linux app 运行在 rel4、sel4 之上

## 如何运行

#### 环境安装

> 可能需要构建文件系统

```shell
dd if=/dev/zero of=mount.img bs=4M count=32
mkfs.ext4 -b 4096 mount.img
```

> 运行

```shell
mkdir -p .env
wget -qO- https://github.com/yfblock/rel4-kernel-autobuild/releases/download/release-2025-01-08/seL4.tar.gz | gunzip | tar -xvf - -C .env --strip-components 1

# Optional
wget https://github.com/rel4team/rust-root-task-demo-mi-dev/raw/refs/heads/new-root-task/busybox

```

> 请确保您的 .env 目录下有 seL4 文件夹

#### 运行

```shell
make run
```
