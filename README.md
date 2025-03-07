# rel4-linux-kit

> rel4 linux kit 提供一套工具和服务以便于 linux app 运行在 rel4、sel4 之上

## 环境安装

#### 安装 musl gcc 工具链

> 请确保您已安装 `aarch64-linux-musl-cross`,如果没有安装可以先执行

```shell
wget https://musl.cc/aarch64-linux-musl-cross.tgz
tar zxf aarch64-linux-musl-cross.tgz
export PATH=$PATH:`pwd`/aarch64-linux-musl-cross/bin
```

#### 下载 sel4 内核

```shell
mkdir -p .env
wget -qO- https://github.com/yfblock/rel4-kernel-autobuild/releases/download/release-2025-03-06/seL4.tar.gz | gunzip | tar -xvf - -C .env --strip-components 1
```

#### 下载并构建测例

```shell
wget -qO- https://github.com/yfblock/rel4-kernel-autobuild/releases/download/release-2025-03-06/aarch64.tgz | tar -xf - -C .env
mkdir -p testcases
./tools/modify-multi.py .env/aarch64 testcases
```

## 运行

```shell
make run

## if you want to get a clean output.
make run LOG=error
```
