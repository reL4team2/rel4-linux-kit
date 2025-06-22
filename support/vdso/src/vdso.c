#include <stdint.h>

// musl中用来保存时间的结构体
struct timespec
{
    uint64_t tv_sec;
    long tv_nsec;
};

struct TimeSpec
{
    uint64_t secs;
    uint32_t nanos;
    uint32_t _align;
};


// 获取内核时间
int __kernel_clock_gettime(uint64_t _id, struct timespec *tp)
{
    uint64_t cntpct, cntfrq;
    asm volatile("mrs %0, CNTPCT_EL0": "=r"(cntpct));
    asm volatile("mrs %0, CNTFRQ_EL0": "=r"(cntfrq));
    
    uint64_t sec = cntpct / cntfrq;
    uint32_t nsec = (cntpct % cntfrq) * 1000000000 / cntfrq;
    tp->tv_sec = sec;
    tp->tv_nsec = nsec;
    return 0;
}

