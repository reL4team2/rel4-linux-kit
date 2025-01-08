# RootTask 

## Slot Usage

前 32 个 Slot 会被父进程使用和填充

```c
/* caps with fixed slot positions in the root CNode */
enum seL4_RootCNodeCapSlots {
    seL4_CapNull                =  0, /* null cap */
    seL4_CapInitThreadTCB       =  1, /* initial thread's TCB cap */
    seL4_CapInitThreadCNode     =  2, /* initial thread's root CNode cap */
    seL4_CapInitThreadVSpace    =  3, /* initial thread's VSpace cap */
    seL4_CapIRQControl          =  4, /* global IRQ controller cap */
    seL4_CapASIDControl         =  5, /* global ASID controller cap */
    seL4_CapInitThreadASIDPool  =  6, /* initial thread's ASID pool cap */
    seL4_CapIOPortControl       =  7, /* global IO port control cap (null cap if not supported) */
    seL4_CapIOSpace             =  8, /* global IO space cap (null cap if no IOMMU support) */
    seL4_CapBootInfoFrame       =  9, /* bootinfo frame cap */
    seL4_CapInitThreadIPCBuffer = 10, /* initial thread's IPC buffer frame cap */
    seL4_CapDomain              = 11, /* global domain controller cap */
    seL4_CapSMMUSIDControl      = 12, /* global SMMU SID controller cap, null cap if not supported */
    seL4_CapSMMUCBControl       = 13, /* global SMMU CB controller cap, null cap if not supported */
    seL4_CapInitThreadSC        = 14, /* initial thread's scheduling context cap, null cap if not supported */
    seL4_CapSMC                 = 15, /* global SMC cap, null cap if not supported */
    seL4_NumInitialCaps         = 16
};
```

以上是 seL4 官方提供的 Slot 使用, 下面我们也约定了一些特定的 Slot 使用

```c
enum seL4_ExtraCapsConvention {
    seL4_ThreadNotification     = 16; /*FIXME: 应该会在之后挪到别的地方*/
    seL4_CapParentEndPoint      = 17; /*与父进程沟通的 EndPoint，主动发给父任务*/
    seL4_CapSelfEndPoint        = 18; /*给当前进程添加的 EndPoint，自己的唯一接收 Slot，别人可以通过这个 Slot 给自己发送信息*/
}
```



## 简单使用介绍

每个任务启动的时候（root-task）除外，每个任务都会拿到两个 EndPoint，一个是在 17，可以主动发送给父进程消息，另一个在 18，是当前任务的接收 slot, 启动的时候， root-task 拥有所有任务的 EndPoint,然后任务开始后，他们如果需要互相沟通，需要通过 ParentEndPoint 向父进程发送 `FindService`，然后父进程会将特定任务的 `Capability` 复制一份，发送给子任务。

任务之间的 Capablity 传输问题，之前直接找到子进程的 Cnode 写入特定的位置，这样不利于代码的整洁和安全，任务之间直接采用 IPC 进行 Capablity 的交换和传输。

## VSpace 对应的 Capability 问题

将 VSpace 对应的 CNode 映射到 CSpace 最高位可以为 1，如果是大页，那么就不在扩展 CSpace。
但是尽量都以小页为单位映射

如何检测 seL4 特定的 slot 是否为空, 
 Checking if CSlots are empty is done by a neat hack: by attempting to move the CSlots onto themselves. This should fail with an error code seL4_FailedLookup if the source CSLot is empty, and an seL4_DeleteFirst if not. 
https://docs.sel4.systems/Tutorials/capabilities.html

在最高位为 0 的情况下就只映射一些基础的 Capability 就可以了。
可以理解为 RootTask 就不在扩展内存和映射内存了，其他任务也有自己的映射方式，就不需要很粗糙的 loop 方式映射了。


## 存在的问题

rust-analyzer 提供的一些语法提示没有办法照顾到 task[i]，

## Capability 的交换


