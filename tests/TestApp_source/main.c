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
#define NULL ((void *)0)
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
typedef struct FILE FILE;
FILE *fopen(const char *, const char *);
int fclose(FILE *);
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

// <unistd.h>
int chdir(const char *);
char *getcwd(char *, size_t);

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
  int res = 0;
  char *str;

  // Test %s
  str = str_format("%s %s", "test", NULL);
  res += strcmp(str, "test (null)");
  free(str);
  // Test %x
  str = str_format("%x", 2042);
  res += strcmp(str, "7fa");
  free(str);
  // Test %d
  str = str_format("%d %3d %03d %.3d %3.3d %03.3d", 5, 5, 5, 5, 5, 5);
  res += strcmp(str, "5   5 005 005 005 005");
  free(str);
  // Test %f
  str = str_format("%f %3f %03f %.3f %3.3f %03.3f", 10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345);
  res += strcmp(str, "10.123450 10.123450 10.123450 10.123 10.123 10.123");
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

int test_getcwd_chdir() {
  char buf[256];
  char *buf2 = getcwd(buf, sizeof buf);
  if (!buf2 || buf2 != buf || strcmp("/", buf))
    return -1;

  if (!chdir("does_not_exist") || !chdir("/does/not/exist"))
    return -1;

  if (chdir("/var/"))
    return -1;

  if (chdir("mobile/Applications"))
    return -1;

  char *buf3 = getcwd(NULL, 0);
  if (!buf3 || strcmp("/var/mobile/Applications", buf3))
    return -1;
  free(buf3);

  char *buf5 = getcwd(buf, 4); // too small
  if (buf5)
    return -1;

  if (chdir(".."))
    return -1;

  char *buf6 = getcwd(buf, sizeof buf);
  if (!buf6 || buf6 != buf || strcmp("/var/mobile", buf6))
    return -1;

  FILE *fake_file = fopen("TestApp", "r"); // doesn't exist in this directory
  if (fake_file) {
    fclose(fake_file);
    return -1;
  }

  if (chdir("Applications/00000000-0000-0000-0000-000000000000/TestApp.app"))
    return -1;

  if (!chdir("TestApp")) // isn't a directory
    return -1;

  FILE *real_file = fopen("TestApp", "r");
  if (!real_file)
    return -1;
  fclose(real_file);

  if (chdir("/"))
    return -1;

  return 0;
}

#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
    FUNC_DEF(test_qsort), FUNC_DEF(test_vsnprintf), FUNC_DEF(test_sscanf),
    FUNC_DEF(test_errno), FUNC_DEF(test_realloc),   FUNC_DEF(test_getcwd_chdir),
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
