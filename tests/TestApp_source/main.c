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
typedef int wchar_t;

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
int swprintf(wchar_t *, size_t, const wchar_t *, ...);
size_t fwrite(const void *, size_t, size_t, FILE *);

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
long strtol(const char *, char **, int);
unsigned long strtoul(const char *, char **, int);
char *realpath(const char *, char *);
size_t mbstowcs(wchar_t *, const char *, size_t);
size_t wcstombs(char *, const wchar_t *, size_t);

// <string.h>
void *memset(void *, int, size_t);
int memcmp(const void *, const void *, size_t);
void *memmove(void *, const void *, size_t);
int strcmp(const char *, const char *);
char *strncpy(char *, const char *, size_t);
char *strncat(char *, const char *, size_t);
size_t strlcpy(char *, const char *, size_t);
char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);
size_t strlen(const char *);
int strncmp(const char *, const char *, size_t);
size_t strcspn(const char *, const char *);

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

// <locale.h>
#define LC_ALL 0
#define LC_COLLATE 1
#define LC_CTYPE 2
#define LC_MONETARY 3
#define LC_NUMERIC 4
#define LC_TIME 5
#define LC_MESSAGES 6
char *setlocale(int category, const char *locale);

// <dirent.h>
typedef struct {
  int _unused;
} DIR;
struct dirent {
  char _unused[21]; // TODO
  char d_name[1024];
};
DIR *opendir(const char *);
struct dirent *readdir(DIR *);
int closedir(DIR *);

// <wchar.h>
int swscanf(const wchar_t *, const wchar_t *, ...);

// `CFBase.h`

typedef const struct _CFAllocator *CFAllocatorRef;
typedef unsigned int CFStringEncoding;
typedef signed long CFIndex;
typedef struct {
  CFIndex location;
  CFIndex length;
} CFRange;
typedef unsigned long CFOptionFlags;
typedef const struct _CFDictionary *CFDictionaryRef;
typedef const struct _CFString *CFStringRef;
typedef const struct _CFString *CFMutableStringRef;

// `CFString.h`

typedef int CFComparisonResult;
typedef unsigned int CFStringCompareFlags;

void CFStringAppendFormat(CFMutableStringRef s, CFDictionaryRef fo,
                          CFStringRef format, ...);
CFMutableStringRef CFStringCreateMutable(CFAllocatorRef alloc, CFIndex max_len);
CFStringRef CFStringCreateWithCString(CFAllocatorRef alloc, const char *cStr,
                                      CFStringEncoding encoding);
CFComparisonResult CFStringCompare(CFStringRef a, CFStringRef b,
                                   CFStringCompareFlags flags);
CFRange CFStringFind(CFStringRef theString, CFStringRef stringToFind,
                     CFOptionFlags compareOptions);

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
  str = str_format("0x%08x", 184638698);
  res += !!strcmp(str, "0x0b015cea");
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
  str = str_format("%f|%8f|%08f|%.f|%8.f|%.3f|%8.3f|%08.3f|%*f|%0*f", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  res += !!strcmp(str, "-10.123450|-10.123450|-10.123450|-10|     -10|-10.123| "
                       "-10.123|-010.123|-10.123450|-10.123450");
  free(str);
  // Test %e
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  res += !!strcmp(
      str, "1.012345e+01|1.012345e+01|1.012345e+01|1e+01|   "
           "1e+01|1.012e+01|1.012e+01|1.012e+01|1.012345e+01|1.012345e+01");
  free(str);
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  res += !!strcmp(
      str,
      "-1.012345e+01|-1.012345e+01|-1.012345e+01|-1e+01|  "
      "-1e+01|-1.012e+01|-1.012e+01|-1.012e+01|-1.012345e+01|-1.012345e+01");
  free(str);
  // Test %g
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  res += !!strcmp(str, "10.1235| 10.1235|010.1235|1e+01|   1e+01|10.1|    "
                       "10.1|000010.1| 10.1235|010.1235");
  free(str);
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  res += !!strcmp(str, "-10.1235|-10.1235|-10.1235|-1e+01|  -1e+01|-10.1|   "
                       "-10.1|-00010.1|-10.1235|-10.1235");
  free(str);
  str = str_format("%f|%8f|%08f|%.f|%8.f|%.3f|%8.3f|%08.3f|%*f|%0*f", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  res += !!strcmp(str, "-10.123450|-10.123450|-10.123450|-10|     -10|-10.123| "
                       "-10.123|-010.123|-10.123450|-10.123450");
  free(str);
  // Test %e
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  res += !!strcmp(
      str, "1.012345e+01|1.012345e+01|1.012345e+01|1e+01|   "
           "1e+01|1.012e+01|1.012e+01|1.012e+01|1.012345e+01|1.012345e+01");
  free(str);
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  res += !!strcmp(
      str,
      "-1.012345e+01|-1.012345e+01|-1.012345e+01|-1e+01|  "
      "-1e+01|-1.012e+01|-1.012e+01|-1.012e+01|-1.012345e+01|-1.012345e+01");
  free(str);
  // Test %g
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  res += !!strcmp(str, "10.1235| 10.1235|010.1235|1e+01|   1e+01|10.1|    "
                       "10.1|000010.1| 10.1235|010.1235");
  free(str);
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  res += !!strcmp(str, "-10.1235|-10.1235|-10.1235|-1e+01|  -1e+01|-10.1|   "
                       "-10.1|-00010.1|-10.1235|-10.1235");
  free(str);
  // Test length modifiers
  str = str_format("%d %ld %lld %u %lu %llu", 10, 100, 4294967296, 10, 100,
                   4294967296);
  res += !!strcmp(str, "10 100 4294967296 10 100 4294967296");
  free(str);

  return res;
}

int test_sscanf() {
  int a, b;
  short c, d;
  float f;
  char str[4];
  int matched = sscanf("1.23", "%d.%d", &a, &b);
  if (!(matched == 2 && a == 1 && b == 23))
    return -1;
  matched = sscanf("abc111.42", "abc%d.%d", &a, &b);
  if (!(matched == 2 && a == 111 && b == 42))
    return -2;
  matched = sscanf("abc", "%d.%d", &a, &b);
  if (matched != 0)
    return -3;
  matched = sscanf("abc,8", "%[^,],%d", str, &b);
  if (!(matched == 2 && strcmp(str, "abc") == 0 && b == 8))
    return -4;
  matched = sscanf("9,10", "%hi,%i", &c, &a);
  if (!(matched == 2 && c == 9 && a == 10))
    return -5;
  matched = sscanf("DUMMY", "%d", &a);
  if (matched != 0)
    return -6;
  matched = sscanf("+10 -10", "%d %d", &a, &b);
  if (!(matched == 2 && a == 10 && b == -10))
    return -7;
  matched = sscanf("+10 -10", "%hd %hd", &c, &d);
  if (!(matched == 2 && c == 10 && d == -10))
    return -9;
  matched = sscanf("3000\\t4", "%d %d", &a, &b);
  if (!(matched == 1 && a == 3000))
    return -10;
  matched = sscanf("0xFF0000", "%08x", &a);
  if (!(matched == 1 && a == 16711680))
    return -11;
  matched = sscanf("ABC\t1\t", "%s %f", str, &f);
  if (!(matched == 2 && strcmp(str, "ABC") == 0 && f == 1.0))
    return -12;
  matched = sscanf("ABC   1\t", "%s\t%f", str, &f);
  if (!(matched == 2 && strcmp(str, "ABC") == 0 && f == 1.0))
    return -13;
  matched = sscanf("MAX\t\t\t48.0\r\n", "%s %f", str, &f);
  if (!(matched == 2 && strcmp(str, "MAX") == 0 && f == 48.0))
    return -14;
  matched = sscanf("011", "%i", &a);
  if (!(matched == 1 && a == 9))
    return -15;
  matched = sscanf("09", "%i", &a);
  if (!(matched == 1 && a == 0))
    return -16;
  return 0;
}

int test_swscanf() {
  int a, b;
  int matched = swscanf(L"1.23", L"%d.%d", &a, &b);
  if (!(matched == 2 && a == 1 && b == 23))
    return -1;
  matched = swscanf(L"str_01", L"str_%2d", &a);
  if (!(matched == 1 && a == 1))
    return -2;
  return 0;
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

int test_strtoul() {
  char *text = "0xcccccccc";
  char *endptr;
  if (strtoul(text, &endptr, 16) != 3435973836 || endptr != text + 10) {
    return -1;
  }
  return 0;
}

#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
#define MAX_LONG 9223372036854775807
#else
#define MAX_LONG 2147483647
#endif

int test_strtol() {
  const char *p = "10 200000000000000000000000000000  30   -40    junk";
  long res[] = {10, MAX_LONG, 30, -40, 0};
  int count = sizeof(res) / sizeof(long);
  for (int i = 0; i < count; i++) {
    char *endp = NULL;
    long l = strtol(p, &endp, 10);
    if (p == endp)
      break;
    p = endp;
    if (res[i] != l) {
      return -(i + 1);
    }
  }
  p = "-";
  long l = strtol(p, NULL, 0);
  if (l != 0) {
    return -count;
  }
  p = "+";
  l = strtol(p, NULL, 0);
  if (l != 0) {
    return -(count + 1);
  }
  p = "+-+";
  l = strtol(p, NULL, 0);
  if (l != 0) {
    return -(count + 2);
  }
  p = "0x123 +0x123 -0x123";
  long res2[] = {291, 291, -291};
  int count2 = sizeof(res2) / sizeof(long);
  for (int i = 0; i < count2; i++) {
    char *endp = NULL;
    l = strtol(p, &endp, 16);
    if (p == endp)
      break;
    p = endp;
    if (res2[i] != l) {
      return -(count + 2 + i + 1);
    }
  }
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
  if (shared_int != 1) {
    return -1;
  }

  // Check that reopen is fine
  semaphore = sem_open("sem_test", O_CREAT, 0644, 1);
  if (semaphore == SEM_FAILED) {
    printf("Error opening semaphore\n");
    return -1;
  }

  // Sem @ -1
  if (sem_trywait(semaphore) == -1) {
    return -1;
  }

  // Sem still @ -1, should not lock
  if (sem_trywait(semaphore) == 0) {
    return -1;
  }

  // Sem @ 0, should be able to relock
  sem_post(semaphore);
  if (sem_trywait(semaphore) == -1) {
    return -1;
  }

  sem_close(semaphore);
  sem_unlink("sem_test");
  return 0;
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

int test_strlcpy() {
  {
    char src[7] = "origen";
    char dst[15] = "destinodestino";
    char expected[] = "or\0tinodestino";
    int ret = strlcpy(dst, src, 3);
    if (ret != 6 || memcmp(dst, expected, 15)) {
      printf("%d %s\t", ret, dst);
      return 1;
    }
  }

  {
    char src[7] = "origen";
    char dst[15] = "destinodestino";
    char expected[] = "orige\0odestino";
    int ret = strlcpy(dst, src, 6);
    if (ret != 6 || memcmp(dst, expected, 15)) {
      printf("%d %s\t", ret, dst);
      return 2;
    }
  }

  {
    char src[7] = "origen";
    char dst[15] = "destinodestino";
    char expected[] = "origen\0destino";
    int ret = strlcpy(dst, src, 9);
    if (ret != 6 || memcmp(dst, expected, 15)) {
      printf("%d %s\t", ret, dst);
      return 3;
    }
  }

  return 0;
}

int test_setlocale() {
  char *locale;

  // Test getting default locale
  locale = setlocale(LC_ALL, NULL);
  if (strcmp(locale, "C") != 0) {
    return 1;
  }

  // Test setting a locale category
  locale = setlocale(LC_NUMERIC, "es_ES");
  if (strcmp(locale, "es_ES") != 0) {
    return 2;
  }

  // Test if other categories are unaffected
  locale = setlocale(LC_TIME, NULL);
  if (strcmp(locale, "C") != 0) {
    return 3;
  }

  return 0;
}

#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
// assume project dir as cwd
const char *path_test_app = "./tests/TestApp.app";
#else
const char *path_test_app = "/var/mobile/Applications/"
                            "00000000-0000-0000-0000-000000000000/TestApp.app";
#endif

int test_dirent() {
  struct dirent *dp;
  DIR *dirp = opendir(path_test_app);
  if (dirp == NULL) {
    return -1;
  }
  char *contents[] = {"TestApp", "Info.plist", "PkgInfo"};
  int counts[] = {1, 1, 1};
  int total = sizeof(contents) / sizeof(char *);
  while ((dp = readdir(dirp)) != NULL) {
    for (int i = 0; i < total; i++) {
      if (strcmp(contents[i], dp->d_name) == 0) {
        counts[i]--;
        break;
      }
    }
  }
  closedir(dirp);
  for (int i = 0; i < total; i++) {
    if (counts[i] != 0) {
      return -2;
    }
  }
  return 0;
}

int test_strchr() {
  char *src = "abc";
  if (strchr(src, 'a')[0] != 'a' || strrchr(src, 'a')[0] != 'a')
    return -1;
  if (strchr(src, 'b')[0] != 'b' || strrchr(src, 'b')[0] != 'b')
    return -2;
  if (strchr(src, 'c')[0] != 'c' || strrchr(src, 'c')[0] != 'c')
    return -3;
  if (strchr(src, '\0')[0] != '\0' || strrchr(src, '\0')[0] != '\0')
    return -4;
  if (strchr(src, 'd') != NULL || strrchr(src, 'd') != NULL)
    return -5;
  return 0;
}

int test_swprintf() {
  wchar_t wcsbuf[20];
  int res = swprintf(wcsbuf, 20, L"%s", "abc");
  if (res != 3)
    return -1;
  res = swprintf(wcsbuf, 2, L"%d", 510);
  if (res != -1)
    return -2;
  res = swprintf(wcsbuf, 20, L"%S", L"abc");
  if (res != 3)
    return -3;
  return 0;
}

int test_realpath() {
  char buf[256];
  if (chdir(path_test_app))
    return -1;
  // absolute path
  char *res = realpath("/usr", buf);
  if (!res || strcmp(res, "/usr") != 0)
    return -2;
  // relative path
  res = realpath("TestApp", buf);
  char *cwd = getcwd(NULL, 0);
  if (!res || strncmp(cwd, res, strlen(cwd)) != 0 ||
      strncmp("/TestApp", res + strlen(cwd), 8) != 0)
    return -3;
  // `..` and `.` resolution
  res = realpath("../TestApp.app/./TestApp", buf);
  if (!res || strncmp(cwd, res, strlen(cwd)) != 0 ||
      strncmp("/TestApp", res + strlen(cwd), 8) != 0)
    return -4;
  return 0;
}

int test_CFStringFind() {
  CFStringRef a = CFStringCreateWithCString(NULL, "/a/b/c/b", 0x0600);
  CFStringRef b = CFStringCreateWithCString(NULL, "/b", 0x0600);
  CFStringRef d = CFStringCreateWithCString(NULL, "/d", 0x0600);
  // 0 for default options
  CFRange r = CFStringFind(a, b, 0);
  if (!(r.location == 2 && r.length == 2)) {
    return -1;
  }
  // 4 for kCFCompareBackwards
  r = CFStringFind(a, b, 4);
  if (!(r.location == 6 && r.length == 2)) {
    return -2;
  }
  // search string in itself
  r = CFStringFind(a, a, 0);
  if (!(r.location == 0 && r.length == 8)) {
    return -3;
  }
  // search string in itself, backwards
  r = CFStringFind(a, a, 4);
  if (!(r.location == 0 && r.length == 8)) {
    return -4;
  }
  // not found case
  r = CFStringFind(a, d, 0);
  if (!(r.location == -1 && r.length == 0)) {
    return -5;
  }
  // 1 for kCFCompareCaseInsensitive
  CFStringRef b2 = CFStringCreateWithCString(NULL, "/B", 0x0600);
  r = CFStringFind(a, b2, 1);
  if (!(r.location == 2 && r.length == 2)) {
    return -6;
  }
  return 0;
}

int test_strcspn() {
  size_t res = strcspn("abcdef", "abcd");
  if (res != 0) {
    return -1;
  }
  res = strcspn("abcdef", "ef");
  if (res != 4) {
    return -2;
  }
  res = strcspn("abcdef", "");
  if (res != 6) {
    return -3;
  }
  return 0;
}

int test_mbstowcs() {
  wchar_t wbuffer[64];
  char buffer[64];
  size_t res;

  char *test_str = "Hello, World!";
  res = mbstowcs(wbuffer, test_str, 64);
  if (res == (size_t)-1) {
    return -1;
  }

  res = wcstombs(buffer, wbuffer, 64);
  if (res == (size_t)-1) {
    return -2;
  }

  if (strcmp(test_str, buffer) != 0) {
    return -3;
  }

  return 0;
}

int test_CFMutableString() {
  CFMutableStringRef mut_str = CFStringCreateMutable(NULL, 0);
  CFStringRef fmt = CFStringCreateWithCString(NULL, "%d %.2f", 0x0600);
  CFStringAppendFormat(mut_str, NULL, fmt, -100, 3.14);
  CFStringRef res = CFStringCreateWithCString(NULL, "-100 3.14", 0x0600);
  if (CFStringCompare(mut_str, res, 0) != 0) {
    return -1;
  }
  return 0;
}

int test_fwrite() {
  FILE *some_file = fopen("TestApp", "r");
  size_t res = fwrite(NULL, 1, 1, some_file);
  fclose(some_file);
  if (res != 0) {
    return -1;
  }
  return 0;
}

// clang-format off
#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
    FUNC_DEF(test_qsort),
    FUNC_DEF(test_vsnprintf),
    FUNC_DEF(test_sscanf),
    FUNC_DEF(test_swscanf),
    FUNC_DEF(test_errno),
    FUNC_DEF(test_realloc),
    FUNC_DEF(test_atof),
    FUNC_DEF(test_strtof),
    FUNC_DEF(test_getcwd_chdir),
    FUNC_DEF(test_sem),
    FUNC_DEF(test_CGAffineTransform),
    FUNC_DEF(test_strncpy),
    FUNC_DEF(test_strncat),
    FUNC_DEF(test_strlcpy),
    FUNC_DEF(test_setlocale),
    FUNC_DEF(test_strtoul),
    FUNC_DEF(test_strtol),
    FUNC_DEF(test_dirent),
    FUNC_DEF(test_strchr),
    FUNC_DEF(test_swprintf),
    FUNC_DEF(test_realpath),
    FUNC_DEF(test_CFStringFind),
    FUNC_DEF(test_strcspn),
    FUNC_DEF(test_mbstowcs),
    FUNC_DEF(test_CFMutableString),
    FUNC_DEF(test_fwrite),
};
// clang-format on

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
