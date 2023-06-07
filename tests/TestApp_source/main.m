/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
/*
This is a main file for the TestApp which is used for integration testing.
This code supposed to be compiled with iPhone SDK and Xcode 3.1 Developer Tools
for Mac OS X v10.5
*/
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int int_compar(const void *a, const void *b) { return *(int *)a - *(int *)b; }

int sort_and_check(int nel, int *arr, int *expected_arr) {
  qsort(arr, nel, sizeof(int), &int_compar);
  return memcmp(arr, expected_arr, nel * sizeof(int));
}

int test_qsort() {
  // empty
  int res = sort_and_check(0, (int[]){}, (int[]){});
  if (res != 0)
    return -1;
  // one element
  res = sort_and_check(1, (int[]){42}, (int[]){42});
  if (res != 0)
    return -1;
  // even size
  res = sort_and_check(4, (int[]){4, 3, 2, 1}, (int[]){1, 2, 3, 4});
  if (res != 0)
    return -1;
  // odd size
  res =
      sort_and_check(5, (int[]){1, -1, 2, 1024, 4}, (int[]){-1, 1, 2, 4, 1024});
  if (res != 0)
    return -1;
  return 0;
}

char *str_format(const char *format, ...) {
  char *str = malloc(256);
  if (str == NULL) {
    exit(EXIT_FAILURE);
  }
  va_list args;
  va_start(args, format);
  vsnprintf(str, 256, format, args);
  va_end(args);
  return str;
}

int test_vsnprintf() {
  char *str = str_format("%s", "test");
  int res = strcmp(str, "test");
  free(str);
  return res;
}

int main(int argc, char *argv[]) {
  int tests_run = 0;
  int tests_passed = 0;
  printf("test_qsort: ");
  tests_run++;
  if (test_qsort() == 0) {
    printf("OK\n");
    tests_passed++;
  } else {
    printf("FAIL\n");
  }
  printf("test_vsnprintf: ");
  tests_run++;
  if (test_vsnprintf() == 0) {
    printf("OK\n");
    tests_passed++;
  } else {
    printf("FAIL\n");
  }
  printf("Passed %d out of %d tests\n", tests_passed, tests_run);
  return tests_run == tests_passed ? 0 : 1;
}
