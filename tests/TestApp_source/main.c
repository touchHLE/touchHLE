/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This is a main file for the TestApp which is used for integration testing.
// See also tests/README.md and tests/integration.rs for the details of how it
// is compiled and run.

// === Includes ===

// For convenience, let's just include the other source files.

#include "CGAffineTransform.c"

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
double atof(const char *);
float strtof(const char *, char **);

// <string.h>
void *memset(void *, int, size_t);
int memcmp(const void *, const void *, size_t);
void *memmove(void *, const void *, size_t);
int strcmp(const char *, const char *);
char *strncpy(char *, const char *, size_t);
char *strncat(char *, const char *, size_t);

// <unistd.h>
typedef unsigned int __uint32_t;
typedef __uint32_t useconds_t;
int chdir(const char *);
char *getcwd(char *, size_t);
int usleep(useconds_t);

// <fcntl.h>
#define O_CREAT 0x00000200

// <pthread.h>
typedef struct opaque_pthread_t opaque_pthread_t;
typedef struct opaque_pthread_t *__pthread_t;
typedef __pthread_t pthread_t;
typedef struct opaque_pthread_attr_t opaque_pthread_attr_t;
typedef struct opaque_pthread_attr_t *__pthread_attr_t;
typedef __pthread_attr_t pthread_attr_t;
int pthread_create(pthread_t *, const pthread_attr_t *, void *(*)(void *),
                   void *);

// <semaphore.h>
#define SEM_FAILED ((sem_t *)-1)
typedef int sem_t;
int sem_close(sem_t *);
sem_t *sem_open(const char *, int, ...);
int sem_post(sem_t *);
int sem_trywait(sem_t *);
int sem_unlink(const char *);
int sem_wait(sem_t *);

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
  str = str_format("%s", "test");
  res += !!strcmp(str, "test");
  free(str);
  // Test %s NULL
  str = str_format("%s", NULL);
  res += !!strcmp(str, "(null)");
  free(str);
  // Test %x
  str = str_format("%x", 2042);
  res += !!strcmp(str, "7fa");
  free(str);
  // Test %d
  str = str_format("%d|%8d|%08d|%.d|%8.d|%.3d|%8.3d|%08.3d|%*d|%0*d", 5, 5, 5,
                   5, 5, 5, 5, 5, 8, 5, 8, 5);
  res += !!strcmp(
      str,
      "5|       5|00000005|5|       5|005|     005|     005|       5|00000005");
  free(str);
  // Test %f
  str = str_format("%f|%8f|%08f|%.f|%8.f|%.3f|%8.3f|%08.3f|%*f|%0*f", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  res += !!strcmp(str, "10.123450|10.123450|10.123450|10|      10|10.123|  "
                       "10.123|0010.123|10.123450|10.123450");
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

int test_atof() {
  if (atof("1") != 1)
    return -1;
  if (atof("-1") != -1)
    return -2;
  if (atof("01") != 1)
    return -3;
  if (atof("-01") != -1)
    return -4;
  if (atof("10") != 10)
    return -5;
  if (atof("-10") != -10)
    return -6;
  if (atof("010") != 10)
    return -7;
  if (atof("-010") != -10)
    return -8;
  if (atof("1.0") != 1)
    return -9;
  if (atof("-1.0") != -1)
    return -10;
  if (atof("01.0") != 1)
    return -11;
  if (atof("-01.0") != -1)
    return -12;
  if (atof("10.0") != 10)
    return -13;
  if (atof("-10.0") != -10)
    return -14;
  if (atof("010.0") != 10)
    return -15;
  if (atof("-010.0") != -10)
    return -16;
  if (atof("1.5") != 1.5)
    return -17;
  if (atof("-1.5") != -1.5)
    return -18;
  if (atof("01.5") != 1.5)
    return -19;
  if (atof("-01.5") != -1.5)
    return -20;
  if (atof("10.5") != 10.5)
    return -21;
  if (atof("-10.5") != -10.5)
    return -22;
  if (atof("010.5") != 10.5)
    return -23;
  if (atof("-010.5") != -10.5)
    return -24;
  if (atof("  +123.456e7with text right after") != 1234560000)
    return -25;
  if (atof("Text before a number 123.456") != 0)
    return -26;
  return 0;
}

int test_strtof() {
  char *text = "1";
  char *endptr;
  if (strtof(text, &endptr) != 1.0 || endptr != text + 1)
    return -1;
  text = "-1";
  if (strtof(text, &endptr) != -1.0 || endptr != text + 2)
    return -2;
  text = "01";
  if (strtof(text, &endptr) != 1.0 || endptr != text + 2)
    return -3;
  text = "-01";
  if (strtof(text, &endptr) != -1.0 || endptr != text + 3)
    return -4;
  text = "10";
  if (strtof(text, &endptr) != 10.0 || endptr != text + 2)
    return -5;
  text = "-10";
  if (strtof(text, &endptr) != -10.0 || endptr != text + 3)
    return -6;
  text = "010";
  if (strtof(text, &endptr) != 10.0 || endptr != text + 3)
    return -7;
  text = "-010";
  if (strtof(text, &endptr) != -10.0 || endptr != text + 4)
    return -8;
  text = "1.0";
  if (strtof(text, &endptr) != 1.0 || endptr != text + 3)
    return -9;
  text = "-1.0";
  if (strtof(text, &endptr) != -1.0 || endptr != text + 4)
    return -10;
  text = "01.0";
  if (strtof(text, &endptr) != 1.0 || endptr != text + 4)
    return -11;
  text = "-01.0";
  if (strtof(text, &endptr) != -1.0 || endptr != text + 5)
    return -12;
  text = "10.0";
  if (strtof(text, &endptr) != 10.0 || endptr != text + 4)
    return -13;
  text = "-10.0";
  if (strtof(text, &endptr) != -10.0 || endptr != text + 5)
    return -14;
  text = "010.0";
  if (strtof(text, &endptr) != 10.0 || endptr != text + 5)
    return -15;
  text = "-010.0";
  if (strtof(text, &endptr) != -10.0 || endptr != text + 6)
    return -16;
  text = "1.5";
  if (strtof(text, &endptr) != 1.5 || endptr != text + 3)
    return -17;
  text = "-1.5";
  if (strtof(text, &endptr) != -1.5 || endptr != text + 4)
    return -18;
  text = "01.5";
  if (strtof(text, &endptr) != 1.5 || endptr != text + 4)
    return -19;
  text = "-01.5";
  if (strtof(text, &endptr) != -1.5 || endptr != text + 5)
    return -20;
  text = "10.5";
  if (strtof(text, &endptr) != 10.5 || endptr != text + 4)
    return -21;
  text = "-10.5";
  if (strtof(text, &endptr) != -10.5 || endptr != text + 5)
    return -22;
  text = "010.5";
  if (strtof(text, &endptr) != 10.5 || endptr != text + 5)
    return -23;
  text = "-010.5";
  if (strtof(text, &endptr) != -10.5 || endptr != text + 6)
    return -24;
  text = "  +123.456e7with text right after";
  if (strtof(text, &endptr) != 1234560000.0 || endptr != text + 12)
    return -25;
  text = "Text before a number 123.456";
  if (strtof(text, &endptr) != 0.0 || endptr != text + 0)
    return -26;
  text = "1.5";
  if (strtof(text, NULL) != 1.5)
    return -27;
  return 0;
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

sem_t *semaphore;
int shared_int = 0;

void sem_thread_func() {
  while (1) {
    if (sem_trywait(semaphore) == -1) {
      return;
    }
    shared_int = -1;
    sem_post(semaphore);
    usleep(100);
  }
}

int test_sem() {
  semaphore = sem_open("sem_test", O_CREAT, 0644, 1);
  if (semaphore == SEM_FAILED) {
    printf("Error opening semaphore\n");
    return -1;
  }

  pthread_t *my_thread = (pthread_t *)malloc(sizeof(pthread_t));
  pthread_create(my_thread, NULL, (void *)sem_thread_func, NULL);
  usleep(200);

  sem_wait(semaphore);

  shared_int = 1;
  usleep(200);

  sem_close(semaphore);
  sem_unlink("sem_test");

  return shared_int == 1 ? 0 : -1;
}

int test_strncpy() {
  char *src = "test\0abcd";
  char dst[10];
  char *retval;

  char expected1[] = "test\x00\x7F\x7F\x7F\x7F\x7F";
  memset(dst, 0x7F, 10);
  retval = strncpy(dst, src, 5);
  if (retval != dst || memcmp(retval, expected1, 10))
    return 1;

  char expected2[] = "te\x7F\x7F\x7F\x7F\x7F\x7F\x7F\x7F";
  memset(dst, 0x7F, 10);
  retval = strncpy(dst, src, 2);
  if (retval != dst || memcmp(retval, expected2, 10))
    return 2;

  char expected3[] = "test\x00\x00\x00\x00\x00\x00";
  memset(dst, 0x7F, 10);
  retval = strncpy(dst, src, 10);
  if (retval != dst || memcmp(retval, expected3, 10))
    return 3;

  return 0;
}

int test_strncat() {
  {
    char uno[] = "uno\0zzzz";
    char dos[] = "dos\0ZZZZ";

    char expected[] = "unodos\0z";
    char *new = strncat(uno, dos, 100);
    if (new != uno || memcmp(new, expected, 8))
      return 1;
  }

  {
    char uno[] = "uno\0zzzz";
    char dos[] = "dos\0ZZZZ";

    char expected[] = "unod\0zzz";
    char *new = strncat(uno, dos, 1);
    if (new != uno || memcmp(new, expected, 8))
      return 2;
  }

  {
    char uno[] = "uno\0zzzz";
    char dos[] = "dosZZZZZ";

    char expected[] = "unodos\0z";
    char *new = strncat(uno, dos, 3);
    if (new != uno || memcmp(new, expected, 8))
      return 3;
  }

  return 0;
}

#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
    FUNC_DEF(test_qsort),   FUNC_DEF(test_vsnprintf),
    FUNC_DEF(test_sscanf),  FUNC_DEF(test_errno),
    FUNC_DEF(test_realloc), FUNC_DEF(test_atof),
    FUNC_DEF(test_strtof),  FUNC_DEF(test_getcwd_chdir),
    FUNC_DEF(test_sem),     FUNC_DEF(test_CGAffineTransform),
    FUNC_DEF(test_strncpy), FUNC_DEF(test_strncat),
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
    int latest_test_result = test_func_array[i].func();
    if (latest_test_result == 0) {
      printf("OK\n");
      tests_passed++;
    } else {
      printf("FAIL (%d)\n", latest_test_result);
    }
  }

  printf("Passed %d out of %d tests\n", tests_passed, tests_run);
  exit(tests_run == tests_passed ? 0 : 1);
}
