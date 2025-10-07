#include <stdio.h>
#include <stdlib.h>

int main() {
  double before = 1.0;
  int t = fork();
  // for (int i = 0; i < 5; i++) {
  //   int t = fork();
  //   if (t == 0)
  //     break;
  // }
  double a, b, c;
  for (;;) {
    a = (double)rand();
    // for (int i = 0; i < 0x1000000; i++)
    //   __asm__("nop");
    b = (double)rand();
    c = a + b;
    printf("value equals %f = %f + %f \n", c, a, b);
    // if (c != a + b) {
    //   printf("value not equals %f = %f + %f \n", c, a, b);
    //   exit(-1);
    // }
  }
  return 0;
}
