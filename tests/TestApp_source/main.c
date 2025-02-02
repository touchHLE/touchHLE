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
char *strdup(const char *);

// <unistd.h>
typedef unsigned int __uint32_t;
typedef __uint32_t useconds_t;
int chdir(const char *);
char *getcwd(char *, size_t);
int usleep(useconds_t);

// <fcntl.h>
#define O_RDONLY 0x00000000
#define O_WRONLY 0x00000001
#define O_RDWR 0x00000002
#define O_CREAT 0x00000200

int open(const char *, int, ...);
int close(int);

// <pthread.h>
typedef struct opaque_pthread_t opaque_pthread_t;
typedef struct opaque_pthread_t *__pthread_t;
typedef __pthread_t pthread_t;

typedef struct opaque_pthread_attr_t opaque_pthread_attr_t;
typedef struct opaque_pthread_attr_t *__pthread_attr_t;
typedef __pthread_attr_t pthread_attr_t;

struct _opaque_pthread_mutex_t {
  long __sig;
  char __opaque[40];
};
typedef struct _opaque_pthread_mutex_t __pthread_mutex_t;
typedef __pthread_mutex_t pthread_mutex_t;

typedef struct opaque_pthread_mutexattr_t opaque_pthread_mutexattr_t;
typedef struct opaque_pthread_mutexattr_t *__pthread_mutexattr_t;
typedef __pthread_mutexattr_t pthread_mutexattr_t;

typedef struct opaque_pthread_cond_t opaque_pthread_cond_t;
typedef struct opaque_pthread_cond_t *__pthread_cond_t;
typedef __pthread_cond_t pthread_cond_t;

typedef struct opaque_pthread_condattr_t opaque_pthread_condattr_t;
typedef struct opaque_pthread_condattr_t *__pthread_condattr_t;
typedef __pthread_condattr_t pthread_condattr_t;

int pthread_create(pthread_t *, const pthread_attr_t *, void *(*)(void *),
                   void *);

int pthread_cond_init(pthread_cond_t *, const pthread_condattr_t *);
int pthread_cond_signal(pthread_cond_t *);
int pthread_cond_wait(pthread_cond_t *, pthread_mutex_t *);

int pthread_mutex_init(pthread_mutex_t *, const pthread_mutexattr_t *);
int pthread_mutex_lock(pthread_mutex_t *);
int pthread_mutex_unlock(pthread_mutex_t *);

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

#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
typedef long _register_t; // 64-bit definition
#else
typedef int _register_t;
#endif

// <setjmp.h>
#define _JBLEN (10 + 16 + 2)
typedef _register_t jmp_buf[_JBLEN];
int setjmp(jmp_buf env);
void longjmp(jmp_buf env, int val);

// <ctype.h>
int __maskrune(wchar_t, unsigned long);

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
int scandir(const char *, struct dirent ***, int (*)(struct dirent *),
            int (*)(const void *, const void *));

// <wchar.h>
int swscanf(const wchar_t *, const wchar_t *, ...);

// <math.h>
long int lrint(double);
long int lrintf(float);
double ldexp(double, int);
float ldexpf(float, int);
float frexpf(float, int *);

// `CFBase.h`

typedef unsigned char Boolean;
typedef const void *CFTypeRef;
typedef const struct _CFAllocator *CFAllocatorRef;
typedef unsigned int CFStringEncoding;
typedef unsigned long CFHashCode;
typedef signed long CFIndex;
typedef struct {
  CFIndex location;
  CFIndex length;
} CFRange;
typedef unsigned long CFOptionFlags;
typedef const struct _CFDictionary *CFDictionaryRef;
typedef const struct _CFString *CFStringRef;
typedef const struct _CFString *CFMutableStringRef;

CFTypeRef CFRetain(CFTypeRef cf);
void CFRelease(CFTypeRef cf);
Boolean CFEqual(CFTypeRef cf1, CFTypeRef cf2);
CFHashCode CFHash(CFTypeRef cf);

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

// `CFDictionary.h`

typedef const struct _CFDictionary *CFMutableDictionaryRef;

typedef const void *(*CFDictionaryRetainCallBack)(CFAllocatorRef alloc,
                                                  const void *value);
typedef void (*CFDictionaryReleaseCallBack)(CFAllocatorRef alloc,
                                            const void *val);
typedef CFStringRef (*CFDictionaryCopyDescriptionCallBack)(const void *val);
typedef Boolean (*CFDictionaryEqualCallBack)(const void *val1,
                                             const void *val2);
typedef CFHashCode (*CFDictionaryHashCallBack)(const void *val);

typedef struct {
  CFIndex version;
  CFDictionaryRetainCallBack retain;
  CFDictionaryReleaseCallBack release;
  CFDictionaryCopyDescriptionCallBack copyDescription;
  CFDictionaryEqualCallBack equal;
  CFDictionaryHashCallBack hash;
} CFDictionaryKeyCallBacks;

typedef struct {
  CFIndex version;
  CFDictionaryRetainCallBack retain;
  CFDictionaryReleaseCallBack release;
  CFDictionaryCopyDescriptionCallBack copyDescription;
  CFDictionaryEqualCallBack equal;
} CFDictionaryValueCallBacks;

CFMutableDictionaryRef
CFDictionaryCreateMutable(CFAllocatorRef allocator, CFIndex capacity,
                          const CFDictionaryKeyCallBacks *keyCallBacks,
                          const CFDictionaryValueCallBacks *valueCallBacks);
void CFDictionaryAddValue(CFMutableDictionaryRef dict, const void *key,
                          const void *value);
void CFDictionarySetValue(CFMutableDictionaryRef dict, const void *key,
                          const void *value);
void CFDictionaryRemoveValue(CFMutableDictionaryRef dict, const void *key);
void CFDictionaryRemoveAllValues(CFMutableDictionaryRef dict);
const void *CFDictionaryGetValue(CFDictionaryRef dict, const void *key);
CFIndex CFDictionaryGetCount(CFDictionaryRef dict);
void CFDictionaryGetKeysAndValues(CFDictionaryRef dict, const void **keys,
                                  const void **values);

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
  // Test % without a specifier
  str = str_format("abc%");
  res += !!strcmp(str, "abc");
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
  // Test %g with trailing zeros
  str = str_format("%.14g", 1.0);
  res += !!strcmp(str, "1");
  free(str);
  // Test %g with a precision argument
  str = str_format("%.*g", 4, 10.234);
  res += !!strcmp(str, "10.23");
  free(str);
  // Test length modifiers
  str = str_format("%d %ld %lld %qd %u %lu %llu %qu", 10, 100, 4294967296,
                   4294967296, 10, 100, 4294967296, 4294967296);
  res += !!strcmp(str,
                  "10 100 4294967296 4294967296 10 100 4294967296 4294967296");
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

int done = 0;
pthread_mutex_t m;
pthread_cond_t c;

void thr_exit() {
  pthread_mutex_lock(&m);
  done = 1;
  pthread_cond_signal(&c);
  pthread_mutex_unlock(&m);
}

void *child(void *arg) {
  thr_exit();
  return NULL;
}

void thr_join() {
  pthread_mutex_lock(&m);
  while (done == 0) {
    pthread_cond_wait(&c, &m);
  }
  pthread_mutex_unlock(&m);
}

int test_cond_var() {
  pthread_t p;

  pthread_mutex_init(&m, NULL);
  pthread_cond_init(&c, NULL);

  pthread_create(&p, NULL, child, NULL);
  thr_join();

  return done == 1 ? 0 : -1;
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

int test_scandir() {
  struct dirent **namelist;
  int n = scandir(path_test_app, &namelist, NULL, NULL);
  if (n < 0) {
    return -1;
  }
  char *contents[] = {"TestApp", "Info.plist", "PkgInfo"};
  int counts[] = {1, 1, 1};
  int total = sizeof(contents) / sizeof(char *);
  while (n--) {
    for (int i = 0; i < total; i++) {
      if (strcmp(contents[i], namelist[n]->d_name) == 0) {
        counts[i]--;
        break;
      }
    }
    free(namelist[n]);
  }
  free(namelist);
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
  CFStringRef a = CFStringCreateWithCString(NULL, "/a/b/c/b", 0x600);
  CFStringRef b = CFStringCreateWithCString(NULL, "/b", 0x600);
  CFStringRef d = CFStringCreateWithCString(NULL, "/d", 0x600);
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

int test_open() {
  int fd;
  // Test opening directories
  fd = open("/usr", O_RDONLY);
  if (fd == -1) {
    return -1;
  }
  close(fd);

  fd = open("/usr", O_WRONLY);
  if (fd != -1) {
    close(fd);
    return -2;
  }

  fd = open("/usr", O_RDWR);
  if (fd != -1) {
    close(fd);
    return -3;
  }

  return 0;
}

int test_CFMutableDictionary_NullCallbacks() {
  CFMutableDictionaryRef dict = CFDictionaryCreateMutable(NULL, 0, NULL, NULL);
  if (dict == NULL) {
    return -1;
  }

  const char *key = "Key";
  const char *value = "Value";
  CFDictionaryAddValue(dict, key, value);
  const void *retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != value) {
    CFRelease(dict);
    return -2;
  }

  const char *valueNew = "NewValue";
  CFDictionaryAddValue(dict, key, valueNew);
  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != value) {
    CFRelease(dict);
    return -3;
  }

  CFDictionarySetValue(dict, key, NULL);
  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != NULL) {
    CFRelease(dict);
    return -4;
  }

  CFDictionarySetValue(dict, key, valueNew);
  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != valueNew) {
    CFRelease(dict);
    return -5;
  }

  CFDictionaryRemoveValue(dict, key);
  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != NULL) {
    CFRelease(dict);
    return -6;
  }

  CFDictionaryAddValue(dict, key, value);
  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != value) {
    CFRelease(dict);
    return -7;
  }

  CFIndex count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -8;
  }

  const void **keys = malloc(sizeof(char *) * count);
  const void **values = malloc(sizeof(char *) * count);
  CFDictionaryGetKeysAndValues(dict, keys, values);
  if (keys[0] != key || values[0] != value) {
    free(keys);
    free(values);
    CFRelease(dict);
    return -9;
  }
  free(keys);
  free(values);

  CFDictionaryRemoveAllValues(dict);
  count = CFDictionaryGetCount(dict);
  if (count != 0) {
    CFRelease(dict);
    return -10;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 0) {
    CFRelease(dict);
    return -11;
  }

  CFRelease(dict);
  return 0;
}

// Counters for checking key/value callbacks
static int keyRetainCount = 0;
static int keyReleaseCount = 0;
static int keyEqualCount = 0;
static int keyHashCount = 0;
static int valueRetainCount = 0;
static int valueReleaseCount = 0;
static int valueEqualCount = 0;

// Custom CFDictionary key/value callbacks
const void *TestKeyRetain(CFAllocatorRef allocator, const void *value) {
  keyRetainCount++;
  if (value == NULL) {
    return NULL;
  }
  return strdup((const char *)value);
}
void TestKeyRelease(CFAllocatorRef allocator, const void *value) {
  keyReleaseCount++;
  if (value == NULL) {
    return;
  }
  free((void *)value);
}
Boolean TestKeyEqual(const void *value1, const void *value2) {
  keyEqualCount++;
  if (value1 == value2) {
    return 1;
  }
  if (value1 == NULL || value2 == NULL) {
    return 0;
  }
  return strcmp((const char *)value1, (const char *)value2) == 0;
}
CFHashCode TestKeyHash(const void *value) {
  keyHashCount++;
  return (value == NULL) ? 0 : 5;
}
const void *TestValueRetain(CFAllocatorRef allocator, const void *value) {
  valueRetainCount++;
  return (value == NULL) ? NULL : strdup((const char *)value);
}
void TestValueRelease(CFAllocatorRef allocator, const void *value) {
  valueReleaseCount++;
  if (value == NULL) {
    return;
  }
  free((void *)value);
}
Boolean TestValueEqual(const void *value1, const void *value2) {
  valueEqualCount++;
  if (value1 == value2) {
    return 1;
  }
  if (value1 == NULL || value2 == NULL) {
    return 0;
  }
  return strcmp((const char *)value1, (const char *)value2) == 0;
}
CFDictionaryKeyCallBacks testKeyCallBacks = {0, // version
                                             TestKeyRetain,
                                             TestKeyRelease,
                                             NULL,
                                             TestKeyEqual,
                                             TestKeyHash};
CFDictionaryValueCallBacks testValueCallBacks = {
    0, // version
    TestValueRetain, TestValueRelease, NULL, TestValueEqual};

int test_CFMutableDictionary_CustomCallbacks_PrimitiveTypes() {
  // Reset counters
  keyRetainCount = keyReleaseCount = keyEqualCount = keyHashCount = 0;
  valueRetainCount = valueReleaseCount = valueEqualCount = 0;

  CFMutableDictionaryRef dict = CFDictionaryCreateMutable(
      NULL, 0, &testKeyCallBacks, &testValueCallBacks);
  if (dict == NULL) {
    return -1;
  }

  CFIndex count = CFDictionaryGetCount(dict);
  if (count != 0) {
    CFRelease(dict);
    return -2;
  }

  const char *key = "Key";
  const char *value = "Value";
  CFDictionaryAddValue(dict, key, value);

  // Hash key function should be called at least once
  if (keyRetainCount != 1 || keyHashCount < 1 || valueRetainCount != 1) {
    CFRelease(dict);
    return -3;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -4;
  }

  const void *retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue == NULL) {
    CFRelease(dict);
    return -5;
  }
  if (strcmp((const char *)retrievedValue, value) != 0) {
    CFRelease(dict);
    return -6;
  }
  if (keyEqualCount < 1) {
    CFRelease(dict);
    return -7;
  }

  const char *valueNew = "NewValue";
  CFDictionaryAddValue(dict, key, valueNew);
  // The key already exists, so the value should not be added
  if (keyRetainCount != 1 || valueRetainCount != 1) {
    CFRelease(dict);
    return -8;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -9;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (strcmp((const char *)retrievedValue, value) != 0) {
    CFRelease(dict);
    return -10;
  }

  CFDictionarySetValue(dict, key, NULL);
  if (valueReleaseCount != 1 || valueRetainCount != 2) {
    CFRelease(dict);
    return -11;
  }

  // Check that count is 1 after setting value to NULL
  // (NULL is a valid value for CFDictionary!)
  count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -12;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != NULL) {
    CFRelease(dict);
    return -13;
  }
  if (keyReleaseCount != 1 || valueReleaseCount != 1) {
    CFRelease(dict);
    return -14;
  }

  CFDictionarySetValue(dict, key, valueNew);
  if (keyReleaseCount != 2 || valueReleaseCount != 2) {
    CFRelease(dict);
    return -15;
  }
  if (valueRetainCount != 3) {
    CFRelease(dict);
    return -16;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -17;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue == NULL ||
      strcmp((const char *)retrievedValue, valueNew) != 0) {
    CFRelease(dict);
    return -18;
  }
  if (keyReleaseCount != 2 || valueReleaseCount != 2) {
    CFRelease(dict);
    return -19;
  }

  CFDictionaryRemoveValue(dict, key);
  if (keyReleaseCount != 3 || valueReleaseCount != 3) {
    CFRelease(dict);
    return -20;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 0) {
    CFRelease(dict);
    return -21;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != NULL) {
    CFRelease(dict);
    return -22;
  }
  if (keyRetainCount != 3 || valueRetainCount != 3) {
    CFRelease(dict);
    return -23;
  }

  CFDictionaryAddValue(dict, key, value);
  if (keyRetainCount != 4 || valueRetainCount != 4) {
    CFRelease(dict);
    return -24;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -25;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue == NULL ||
      strcmp((const char *)retrievedValue, value) != 0) {
    CFRelease(dict);
    return -26;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(dict);
    return -27;
  }

  const void **keys = malloc(sizeof(void *) * count);
  const void **values = malloc(sizeof(void *) * count);
  CFDictionaryGetKeysAndValues(dict, keys, values);
  if (strcmp((const char *)keys[0], key) != 0 ||
      strcmp((const char *)values[0], value) != 0) {
    free(keys);
    free(values);
    CFRelease(dict);
    return -28;
  }
  free(keys);
  free(values);
  if (keyReleaseCount != 3 || valueReleaseCount != 3) {
    CFRelease(dict);
    return -29;
  }

  CFDictionaryRemoveAllValues(dict);
  if (keyReleaseCount != 4 || valueReleaseCount != 4) {
    CFRelease(dict);
    return -30;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 0) {
    CFRelease(dict);
    return -31;
  }

  // Check that value equality callback was not called (based on macOS behavior)
  if (valueEqualCount != 0) {
    CFRelease(dict);
    return -32;
  }

  CFRelease(dict);
  return 0;
}

// Counters for retain and release.
//
// We couldn't relay on the retainCounts of the objects directly
// as Objective-C retainCount method is meant to be for debug
// purposes only and modern versions are using tagged pointers anyway,
// thus return value of this method can be meaningless.
// Instead, we hook counter to the retain/release callbacks
// and check for changes in deltas
// (because actual counts could be different between implementations).
static int retainCount = 0;
static int releaseCount = 0;

// Callbacks similar to kCFTypeDictionaryKeyCallBacks and
// kCFTypeDictionaryValueCallBacks
const void *CFRetainWrapper(CFAllocatorRef allocator, const void *value) {
  retainCount++;
  return CFRetain(value);
}

void CFReleaseWrapper(CFAllocatorRef allocator, const void *value) {
  releaseCount++;
  CFRelease(value);
}
CFHashCode CFHashWrapper(const void *value) { return CFHash(value); }
Boolean CFEqualWrapper(const void *value1, const void *value2) {
  return CFEqual(value1, value2);
}
CFDictionaryKeyCallBacks testDefaultKeyCallBacks = {
    0, // version
    CFRetainWrapper,
    CFReleaseWrapper,
    NULL, // stub of CFCopyDescription
    CFEqualWrapper,
    CFHashWrapper};
CFDictionaryValueCallBacks testDefaultValueCallBacks = {
    0, // version
    CFRetainWrapper, CFReleaseWrapper,
    NULL, // stub of CFCopyDescription
    CFEqualWrapper};

int test_CFMutableDictionary_CustomCallbacks_CFTypes() {
  // Reset counters
  retainCount = 0;
  releaseCount = 0;

  CFMutableDictionaryRef dict = CFDictionaryCreateMutable(
      NULL, 0, &testDefaultKeyCallBacks, &testDefaultValueCallBacks);
  if (dict == NULL) {
    return -1;
  }

  CFStringRef key = CFStringCreateWithCString(NULL, "Key", 0x600);
  CFStringRef value = CFStringCreateWithCString(NULL, "Value", 0x600);
  if (key == NULL || value == NULL) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(dict);
    return -2;
  }

  // Create copies to be stored in the dictionary
  CFStringRef key1 = CFStringCreateWithCString(NULL, "Key", 0x600);
  CFStringRef value1 = CFStringCreateWithCString(NULL, "Value", 0x600);

  int retainCountBefore = retainCount;
  int releaseCountBefore = releaseCount;

  CFDictionaryAddValue(dict, key1, value1);

  int deltaRetain = retainCount - retainCountBefore;
  int deltaRelease = releaseCount - releaseCountBefore;
  // For the purpose of this test, we only care about delta between
  // retain and release counts, e.g. receiving 1 retain and 1 release
  // has the same net effect as receiving 2 retains and 2 releases,
  // as delta for both of them is 0
  int globalDelta = deltaRetain - deltaRelease;

  if (globalDelta != 2) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(key1);
    CFRelease(value1);
    CFRelease(dict);
    return -3;
  }

  // Release key1 and value1 since the dictionary has retained them
  CFRelease(key1);
  CFRelease(value1);

  const void *retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue == NULL) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(dict);
    return -4;
  }
  if (!CFEqual((CFStringRef)retrievedValue, value)) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(dict);
    return -5;
  }

  CFStringRef valueNew = CFStringCreateWithCString(NULL, "NewValue", 0x600);
  if (valueNew == NULL) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(dict);
    return -6;
  }

  retainCountBefore = retainCount;
  releaseCountBefore = releaseCount;

  CFDictionaryAddValue(dict, key, valueNew);

  // Since the key already exists, the new value should not be added
  deltaRetain = retainCount - retainCountBefore;
  deltaRelease = releaseCount - releaseCountBefore;
  globalDelta = deltaRetain - deltaRelease;

  if (globalDelta != 0) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -7;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (!CFEqual((CFStringRef)retrievedValue, value)) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -8;
  }

  retainCountBefore = retainCount;
  releaseCountBefore = releaseCount;

  CFDictionarySetValue(dict, key, valueNew);

  deltaRetain = retainCount - retainCountBefore;
  deltaRelease = releaseCount - releaseCountBefore;
  globalDelta = deltaRetain - deltaRelease;

  if (globalDelta != 0) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -9;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (!CFEqual((CFStringRef)retrievedValue, valueNew)) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -10;
  }

  retainCountBefore = retainCount;
  releaseCountBefore = releaseCount;

  CFDictionaryRemoveValue(dict, key);

  deltaRetain = retainCount - retainCountBefore;
  deltaRelease = releaseCount - releaseCountBefore;
  // The dictionary should release the key and value
  // So delta should be -2
  globalDelta = deltaRetain - deltaRelease;

  if (globalDelta != -2) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -11;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (retrievedValue != NULL) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -12;
  }

  retainCountBefore = retainCount;
  releaseCountBefore = releaseCount;

  CFDictionaryAddValue(dict, key, value);

  deltaRetain = retainCount - retainCountBefore;
  deltaRelease = releaseCount - releaseCountBefore;
  // The dictionary should retain the key and value
  // So delta should be +2
  globalDelta = deltaRetain - deltaRelease;

  if (globalDelta != 2) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -13;
  }

  retrievedValue = CFDictionaryGetValue(dict, key);
  if (!CFEqual((CFStringRef)retrievedValue, value)) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -14;
  }

  CFIndex count = CFDictionaryGetCount(dict);
  if (count != 1) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -15;
  }

  const void **keys = malloc(sizeof(void *) * count);
  const void **values = malloc(sizeof(void *) * count);
  if (keys == NULL || values == NULL) {
    free(keys);
    free(values);
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -16;
  }
  CFDictionaryGetKeysAndValues(dict, keys, values);

  if (!CFEqual((CFStringRef)keys[0], key) ||
      !CFEqual((CFStringRef)values[0], value)) {
    free(keys);
    free(values);
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -17;
  }
  free(keys);
  free(values);

  retainCountBefore = retainCount;
  releaseCountBefore = releaseCount;

  CFDictionaryRemoveAllValues(dict);

  deltaRetain = retainCount - retainCountBefore;
  deltaRelease = releaseCount - releaseCountBefore;
  // The dictionary should release the key and value
  // So delta should be -2
  globalDelta = deltaRetain - deltaRelease;

  if (globalDelta != -2) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -18;
  }

  count = CFDictionaryGetCount(dict);
  if (count != 0) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(valueNew);
    CFRelease(dict);
    return -19;
  }

  CFRelease(key);
  CFRelease(value);
  CFRelease(valueNew);
  CFRelease(dict);

  return 0;
}

int test_lrint() {
  struct {
    double input;
    long int expected;
  } test_cases[] = {
      {0.0, 0L},
      {0.5, 0L},
      {1.0, 1L},
      {1.5, 2L},
      {2.0, 2L},
      {2.5, 2L},
      {3.0, 3L},
      {3.5, 4L},
      {4.5, 4L},
      {5.5, 6L},
      {-0.0, 0L},
      {-0.5, 0L},
      {-1.0, -1L},
      {-1.5, -2L},
      {-2.0, -2L},
      {-2.5, -2L},
      {-3.0, -3L},
      {-3.5, -4L},
      {-4.5, -4L},
      {-5.5, -6L},
      {1.4999999999, 1L},
      {1.5000000001, 2L},
      {-1.4999999999, -1L},
      {-1.5000000001, -2L},
      // Around INT_MAX
      {2147483647.0, 2147483647L},
      {2147483646.5, 2147483646L},
      {2147483647.4, 2147483647L},
      // Around INT_MIN
      {-2147483648.0, -2147483648L},
      {-2147483648.5, -2147483648L},
      {-2147483647.5, -2147483648L},
  };
  int num_tests = sizeof(test_cases) / sizeof(test_cases[0]);
  for (int i = 0; i < num_tests; i++) {
    double input = test_cases[i].input;
    long int expected = test_cases[i].expected;
    long int result = lrint(input);
    if (result != expected) {
      return -(i + 1);
    }
  }

  struct {
    float input;
    long int expected;
  } test_cases_f[] = {
      {0.0f, 0L},
      {0.5f, 0L},
      {1.0f, 1L},
      {1.5f, 2L},
      {2.0f, 2L},
      {2.5f, 2L},
      {3.0f, 3L},
      {3.5f, 4L},
      {4.5f, 4L},
      {5.5f, 6L},
      {-0.0f, 0L},
      {-0.5f, 0L},
      {-1.0f, -1L},
      {-1.5f, -2L},
      {-2.0f, -2L},
      {-2.5f, -2L},
      {-3.0f, -3L},
      {-3.5f, -4L},
      {-4.5f, -4L},
      {-5.5f, -6L},
      {1.4999999f, 1L},
      {1.5000001f, 2L},
      {-1.4999999f, -1L},
      {-1.5000001f, -2L},
#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
      // on macOS `long int` is 8 bytes
      {2147483648.0f, 2147483648L},
#else
      {2147483648.0f, 2147483647L}
#endif
  };
  int num_tests_f = sizeof(test_cases_f) / sizeof(test_cases_f[0]);
  for (int i = 0; i < num_tests_f; i++) {
    float input = test_cases_f[i].input;
    long int expected = test_cases_f[i].expected;
    long int result = lrintf(input);
    if (result != expected) {
      return -(num_tests + i + 1);
    }
  }

  return 0;
}

int test_ldexp() {
  struct {
    double x;
    int n;
    double expected;
  } test_cases[] = {
      {0.0, 5, 0.0},  {-0.0, -3, -0.0}, {1.0, 0, 1.0},   {1.0, 1, 2.0},
      {1.0, -1, 0.5}, {2.5, 3, 20.0},   {3.0, -2, 0.75},
  };
  int num_tests = sizeof(test_cases) / sizeof(test_cases[0]);
  for (int i = 0; i < num_tests; i++) {
    double x = test_cases[i].x;
    int n = test_cases[i].n;
    double expected = test_cases[i].expected;
    double result = ldexp(x, n);

    if (expected != result) {
      return -(i + 1);
    }
  }

  struct {
    float x;
    int n;
    float expected;
  } test_cases_f[] = {
      {0.0f, 5, 0.0f},  {-0.0f, -3, -0.0f}, {1.0f, 0, 1.0f},   {1.0f, 1, 2.0f},
      {1.0f, -1, 0.5f}, {2.5f, 3, 20.0f},   {3.0f, -2, 0.75f},
  };
  int num_tests_f = sizeof(test_cases_f) / sizeof(test_cases_f[0]);
  for (int i = 0; i < num_tests_f; i++) {
    float x = test_cases_f[i].x;
    int n = test_cases_f[i].n;
    float expected = test_cases_f[i].expected;
    float result = ldexpf(x, n);

    if (expected != result) {
      return -(num_tests + i + 1);
    }
  }

  return 0;
}

// Just for readability, similar to _CTYPE_* constants
#define MASK_RUNE_ALPHA 0x00100L
#define MASK_RUNE_CONTROL 0x00200L
#define MASK_RUNE_DIGIT 0x00400L
#define MASK_RUNE_GRAPH 0x00800L
#define MASK_RUNE_LOWER 0x01000L
#define MASK_RUNE_PUNCT 0x02000L
#define MASK_RUNE_SPACE 0x04000L
#define MASK_RUNE_UPPER 0x08000L
#define MASK_RUNE_XDIGIT 0x10000L
#define MASK_RUNE_BLANK 0x20000L
#define MASK_RUNE_PRINT 0x40000L

int test_maskrune() {
  struct {
    char c;
    unsigned long mask;
    int expected;
  } test_cases[] = {
      {'A', MASK_RUNE_ALPHA, 256},    {'A', MASK_RUNE_UPPER, 32768},
      {'A', MASK_RUNE_GRAPH, 2048},   {'A', MASK_RUNE_LOWER, 0},

      {'z', MASK_RUNE_ALPHA, 256},    {'z', MASK_RUNE_LOWER, 4096},
      {'z', MASK_RUNE_GRAPH, 2048},   {'z', MASK_RUNE_UPPER, 0},

      {'5', MASK_RUNE_DIGIT, 1024},   {'5', MASK_RUNE_XDIGIT, 65536},
      {'5', MASK_RUNE_ALPHA, 0},

      {'?', MASK_RUNE_PUNCT, 8192},   {'?', MASK_RUNE_GRAPH, 2048},
      {'?', MASK_RUNE_PRINT, 262144}, {'?', MASK_RUNE_ALPHA, 0},

      {' ', MASK_RUNE_SPACE, 16384},  {' ', MASK_RUNE_BLANK, 131072},
      {' ', MASK_RUNE_PRINT, 262144}, {' ', MASK_RUNE_GRAPH, 0},

      {'\n', MASK_RUNE_CONTROL, 512}, {'\n', MASK_RUNE_PRINT, 0},
      {'\n', MASK_RUNE_GRAPH, 0},

      {'F', MASK_RUNE_XDIGIT, 65536}, {'G', MASK_RUNE_XDIGIT, 0},
  };

  int num_tests = sizeof(test_cases) / sizeof(test_cases[0]);
  for (int i = 0; i < num_tests; i++) {
    char c = test_cases[i].c;
    unsigned long mask = test_cases[i].mask;
    int expected = test_cases[i].expected;
    int result = __maskrune(c, mask);

    if (expected != result) {
      return -(i + 1);
    }
  }
  return 0;
}

int test_frexpf(void) {
  int exp_val;
  float m;

  /* Test 1: 8.0f = 0.5 * 2^4 */
  m = frexpf(8.0f, &exp_val);
  if (m != 0.5f || exp_val != 4)
    return -1;

  /* Test 2: 4.0f = 0.5 * 2^3 */
  m = frexpf(4.0f, &exp_val);
  if (m != 0.5f || exp_val != 3)
    return -2;

  /* Test 3: 0.75f is already normalized: 0.75f * 2^0 = 0.75f */
  m = frexpf(0.75f, &exp_val);
  if (m != 0.75f || exp_val != 0)
    return -3;

  /* Test 4: 1.0f = 0.5 * 2^1 */
  m = frexpf(1.0f, &exp_val);
  if (m != 0.5f || exp_val != 1)
    return -4;

  /* Test 5: 0.125f = 0.5 * 2^-2 */
  m = frexpf(0.125f, &exp_val);
  if (m != 0.5f || exp_val != -2)
    return -5;

  /* Test 6: 0.0f should return 0.0f and exponent 0 */
  m = frexpf(0.0f, &exp_val);
  if (m != 0.0f || exp_val != 0)
    return -6;

  /* Test 7: Negative value, -8.0f = -0.5 * 2^4 */
  m = frexpf(-8.0f, &exp_val);
  if (m != -0.5f || exp_val != 4)
    return -7;

  /* Test 8: -0.0f should be preserved (check with signbit) */
  m = frexpf(-0.0f, &exp_val);
  if (m != 0.0f || exp_val != 0)
    return -8;

  return 0;
}

void jmpfunction(jmp_buf env_buf) { longjmp(env_buf, 432); }

int test_setjmp() {
  int val;
  jmp_buf env_buffer;

  /* save calling environment for longjmp */
  val = setjmp(env_buffer);

  if (val != 0) {
    return val == 432 ? 0 : -2;
  }

  jmpfunction(env_buffer);

  return -1;
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
    FUNC_DEF(test_scandir),
    FUNC_DEF(test_strchr),
    FUNC_DEF(test_swprintf),
    FUNC_DEF(test_realpath),
    FUNC_DEF(test_CFStringFind),
    FUNC_DEF(test_strcspn),
    FUNC_DEF(test_mbstowcs),
    FUNC_DEF(test_CFMutableString),
    FUNC_DEF(test_fwrite),
    FUNC_DEF(test_open),
    FUNC_DEF(test_cond_var),
    FUNC_DEF(test_CFMutableDictionary_NullCallbacks),
    FUNC_DEF(test_CFMutableDictionary_CustomCallbacks_PrimitiveTypes),
    FUNC_DEF(test_CFMutableDictionary_CustomCallbacks_CFTypes),
    FUNC_DEF(test_lrint),
    FUNC_DEF(test_ldexp),
    FUNC_DEF(test_maskrune),
    FUNC_DEF(test_frexpf),
    FUNC_DEF(test_setjmp),
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
