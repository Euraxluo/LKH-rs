//extern "C"{
#include "demo.h"
#include "demo2.h"

//}
#include <stdio.h>

int main() {
  hello();
  printf("%d\n", add(1, 2));

  bye();
  printf("%f\n", multiply(2.5, 3.0));

  printf("%f\n",GetTime());
  return 0;
}