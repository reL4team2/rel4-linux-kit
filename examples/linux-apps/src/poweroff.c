#include <stdio.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <linux/reboot.h>
#include <sys/reboot.h>

void shutdown_linux()
{
    // Sync filesystem before shutdown (避免数据丢失)
    sync();

    // 调用 syscall 执行关机
    syscall(SYS_reboot, LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2, LINUX_REBOOT_CMD_POWER_OFF, NULL);
}

int main()
{
    printf("Shutting down the system...\n");
    shutdown_linux();
    return 0;
}
