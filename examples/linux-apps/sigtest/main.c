#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>

void signal_handler(int signo) {
    printf("Received signal %d\n", signo);
    fflush(stdout);
}

int main() {
    // 注册信号处理函数
    signal(SIGUSR1, signal_handler);

    printf("handler addr: %p\n", signal_handler);
    printf("Process ID: %d\n", getpid());

    // 发送信号给自己
    kill(getpid(), SIGUSR1);

    // 信号返回
    printf("signal test end\n");
    return 0;
}

