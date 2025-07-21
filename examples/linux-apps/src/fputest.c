#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#define testty int

int main() {
  testty before = 1.0;
  testty tid = (testty)fork();
  // testty tid = 1;
  testty a, b, c;
  for (;;) {
    a = (testty)rand();
    // for (int i = 0; i < 0x100000; i++)
    //   __asm__("nop");
    b = (testty)rand();
    c = a + b;
    // printf("thread %f value equals %f = %f + %f\n", tid, c, a, b);
    printf("thread %d value equals %d = %d + %d\n", tid, c, a, b);
  }
  return 0;
}
