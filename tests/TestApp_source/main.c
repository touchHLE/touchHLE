/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This is a main file for the TestApp which is used for integration testing.
// See also tests/README.md and tests/integration.rs for the details of how it
// is compiled and run.

// === Declarations ===

// We don't have any system headers for iPhone OS, so we must declare everything
// ourselves rather than #include'ing.

// <stddef.h>
#define NULL ((void*)0)
typedef unsigned long size_t;

// <errno.h>
int *__error(void);
#define errno (*__error())

// <stdarg.h>
typedef __builtin_va_list va_list;
#define va_start(a, b) __builtin_va_start(a, b)
#define va_arg(a, b) __builtin_va_arg(a, b)
#define va_end(a) __builtin_va_end(a)

// <stdio.h>
int sscanf(const char *, const char *, ...);
int printf(const char *, ...);
int vsnprintf(char *, size_t, const char *, va_list);

// <stdlib.h>
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
void exit(int);
void free(void *);
void *malloc(size_t);
void qsort(void *, size_t, size_t, int (*)(const void *, const void *));
void *realloc(void *, size_t);

// <string.h>
int memcmp(const void *, const void *, size_t);
void *memmove(void *, const void *, size_t);
int strcmp(const char *, const char *);

// === Main code ===

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
  char *str = str_format("%s %x %.3d", "test", 2042, 5);
  int res = strcmp(str, "test 7fa 005");
  free(str);
  return res;
}

int test_sscanf() {
  int a, b;
  int matched = sscanf("1.23", "%d.%d", &a, &b);
  if (!(matched == 2 && a == 1 && b == 23))
    return -1;
  matched = sscanf("abc111.42", "abc%d.%d", &a, &b);
  if (!(matched == 2 && a == 111 && b == 42))
    return -1;
  matched = sscanf("abc", "%d.%d", &a, &b);
  return (matched == 0) ? 0 : -1;
}

int test_errno() { return (errno == 0) ? 0 : -1; }

int test_realloc() {
  void *ptr = realloc(NULL, 32);
  memmove(ptr, "abcd", 4);
  ptr = realloc(ptr, 64);
  int res = memcmp(ptr, "abcd", 4);
  free(ptr);
  return res == 0 ? 0 : -1;
}

#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
    // TODO: re-enable qsort. It currently crashes for some reason.
    FUNC_DEF(test_qsort), FUNC_DEF(test_vsnprintf), FUNC_DEF(test_sscanf),
    FUNC_DEF(test_errno), FUNC_DEF(test_realloc),
};

// Because no libc is linked into this executable, there is no libc entry point
// to call main. Instead, integration.rs tells Clang to set the _main symbol
// as the entry point. (It has to be _main because a C compiler will throw
// away stuff not called by main().) Since this is the true entry point, there's
// no argc or argv and we must call exit() ourselves.
int main() {
  int tests_run = 0;
  int tests_passed = 0;

  int n = sizeof(test_func_array) / sizeof(test_func_array[0]);
  int i;
  for (i = 0; i < n; i++) {
    printf("%s: ", test_func_array[i].name);
    tests_run++;
    if (test_func_array[i].func() == 0) {
      printf("OK\n");
      tests_passed++;
    } else {
      printf("FAIL\n");
    }
  }

  printf("Passed %d out of %d tests\n", tests_passed, tests_run);
  exit(tests_run == tests_passed ? 0 : 1);
}
