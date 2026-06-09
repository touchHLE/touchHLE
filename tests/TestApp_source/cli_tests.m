/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file contains the command-line automated tests. tests/integration.rs
// runs these automatically.

#include <CoreFoundation/CFBase.h>
#include <CoreFoundation/CFBundle.h>
#include <CoreFoundation/CFDictionary.h>
#include <CoreFoundation/CFNumber.h>
#include <CoreFoundation/CFString.h>
#include <CoreFoundation/CFURL.h>
#include <arpa/inet.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <fenv.h>
#include <locale.h>
#include <mach/kern_return.h>
#include <mach/thread_info.h>
#include <malloc/malloc.h>
#include <math.h>
#include <pthread.h>
#include <semaphore.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>
#include <wchar.h>

#import "SyncTester.h"

// TODO: include from <mach/thread_act.h> once available in the common-sdk
extern kern_return_t thread_suspend(mach_port_t target_act);
extern kern_return_t thread_resume(mach_port_t target_act);
extern kern_return_t thread_info(mach_port_t target_act, natural_t flavor,
                                 integer_t *thread_info_out,
                                 natural_t *thread_info_out_cnt);

// Declare test functions from other files.

int test_AutoreleasePool(void);    // AutoReleasePoolTest.m
int test_CGAffineTransform(void);  // CGAffineTransform.c
int test_RespondsToSelector(void); // RespondsToSelector.m
int test_Initialize(void);         // Initialize.m

#ifndef DEFINE_ME_WHEN_BUILDING_ON_MACOS
int test_cpp_virtual_inheritance(void); // CppVirtualInheritance.cpp
#endif

// === Main code ===

int test_CGGeometry() {
  CGRect testRect;
  testRect.origin.x = 2.0;
  testRect.origin.y = 3.0;
  testRect.size.width = 100.0;
  testRect.size.height = 200.0;

  if (!(CGRectGetMinX(testRect) == testRect.origin.x &&
        CGRectGetMinX(testRect) == 2.0))
    return -1;
  if (!(CGRectGetMaxX(testRect) == testRect.origin.x + testRect.size.width &&
        CGRectGetMaxX(testRect) == 102.0))
    return -2;

  if (!(CGRectGetMinY(testRect) == testRect.origin.y &&
        CGRectGetMinY(testRect) == 3.0))
    return -3;

  if (!(CGRectGetMaxY(testRect) == testRect.origin.y + testRect.size.height &&
        CGRectGetMaxY(testRect) == 203.0))
    return -4;

  if (!(CGRectGetHeight(testRect) == testRect.size.height))
    return -5;

  if (!(CGRectGetWidth(testRect) == testRect.size.width))
    return -6;

  return 0;
}

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
  char *str;

  // Test %s
  str = str_format("%s", "test");
  if (strcmp(str, "test") != 0) {
    free(str);
    return -1;
  }
  free(str);
  // Test %s NULL
  str = str_format("%s", NULL);
  if (strcmp(str, "(null)") != 0) {
    free(str);
    return -2;
  }
  free(str);
  // Test % without a specifier
  str = str_format("abc%");
  if (strcmp(str, "abc") != 0) {
    free(str);
    return -3;
  }
  free(str);
  // Test %x
  str = str_format("%x", 2042);
  if (strcmp(str, "7fa") != 0) {
    free(str);
    return -4;
  }
  free(str);
  str = str_format("0x%08x", 184638698);
  if (strcmp(str, "0x0b015cea") != 0) {
    free(str);
    return -5;
  }
  free(str);
  // Test %d
  str = str_format("%d|%8d|%08d|%.d|%8.d|%.3d|%8.3d|%08.3d|%*d|%0*d", 5, 5, 5,
                   5, 5, 5, 5, 5, 8, 5, 8, 5);
  if (strcmp(str, "5|       5|00000005|5|       5|005|     005|     005|       "
                  "5|00000005") != 0) {
    free(str);
    return -6;
  }
  free(str);
  // Test %d with alternative form
  str = str_format("%#.2d", 5);
  if (strcmp(str, "05") != 0) {
    free(str);
    return -7;
  }
  free(str);
  // Test %f
  str = str_format("%f|%8f|%08f|%.f|%8.f|%.3f|%8.3f|%08.3f|%*f|%0*f", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  if (strcmp(str, "10.123450|10.123450|10.123450|10|      10|10.123|  "
                  "10.123|0010.123|10.123450|10.123450") != 0) {
    free(str);
    return -8;
  }
  free(str);
  str = str_format("%f|%8f|%08f|%.f|%8.f|%.3f|%8.3f|%08.3f|%*f|%0*f", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  if (strcmp(str, "-10.123450|-10.123450|-10.123450|-10|     -10|-10.123| "
                  "-10.123|-010.123|-10.123450|-10.123450") != 0) {
    free(str);
    return -9;
  }
  free(str);
  // Test %e
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  if (strcmp(str,
             "1.012345e+01|1.012345e+01|1.012345e+01|1e+01|   "
             "1e+01|1.012e+01|1.012e+01|1.012e+01|1.012345e+01|1.012345e+01") !=
      0) {
    free(str);
    return -10;
  }
  free(str);
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  if (strcmp(str, "-1.012345e+01|-1.012345e+01|-1.012345e+01|-1e+01|  "
                  "-1e+01|-1.012e+01|-1.012e+01|-1.012e+01|-1.012345e+01|-1."
                  "012345e+01") != 0) {
    free(str);
    return -11;
  }
  free(str);
  // Test %g
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  if (strcmp(str, "10.1235| 10.1235|010.1235|1e+01|   1e+01|10.1|    "
                  "10.1|000010.1| 10.1235|010.1235") != 0) {
    free(str);
    return -12;
  }
  free(str);
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  if (strcmp(str, "-10.1235|-10.1235|-10.1235|-1e+01|  -1e+01|-10.1|   "
                  "-10.1|-00010.1|-10.1235|-10.1235") != 0) {
    free(str);
    return -13;
  }
  free(str);
  str = str_format("%f|%8f|%08f|%.f|%8.f|%.3f|%8.3f|%08.3f|%*f|%0*f", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  if (strcmp(str, "-10.123450|-10.123450|-10.123450|-10|     -10|-10.123| "
                  "-10.123|-010.123|-10.123450|-10.123450") != 0) {
    free(str);
    return -14;
  }
  free(str);
  // Test %e
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  if (strcmp(str,
             "1.012345e+01|1.012345e+01|1.012345e+01|1e+01|   "
             "1e+01|1.012e+01|1.012e+01|1.012e+01|1.012345e+01|1.012345e+01") !=
      0) {
    free(str);
    return -15;
  }
  free(str);
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  if (strcmp(str, "-1.012345e+01|-1.012345e+01|-1.012345e+01|-1e+01|  "
                  "-1e+01|-1.012e+01|-1.012e+01|-1.012e+01|-1.012345e+01|-1."
                  "012345e+01") != 0) {
    free(str);
    return -16;
  }
  free(str);
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", 0.0, 0.0,
                   0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 16, 0.0, 16, 0.0);
  if (strcmp(str, "0.000000e+00|0.000000e+00|0.000000e+00|0e+00|   "
                  "0e+00|0.000e+00|0.000e+00|0.000e+00|    "
                  "0.000000e+00|00000.000000e+00") != 0) {
    free(str);
    return -17;
  }
  free(str);
  // Test %g
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", 10.12345,
                   10.12345, 10.12345, 10.12345, 10.12345, 10.12345, 10.12345,
                   10.12345, 8, 10.12345, 8, 10.12345);
  if (strcmp(str, "10.1235| 10.1235|010.1235|1e+01|   1e+01|10.1|    "
                  "10.1|000010.1| 10.1235|010.1235") != 0) {
    free(str);
    return -18;
  }
  free(str);
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", -10.12345,
                   -10.12345, -10.12345, -10.12345, -10.12345, -10.12345,
                   -10.12345, -10.12345, 8, -10.12345, 8, -10.12345);
  if (strcmp(str, "-10.1235|-10.1235|-10.1235|-1e+01|  -1e+01|-10.1|   "
                  "-10.1|-00010.1|-10.1235|-10.1235") != 0) {
    free(str);
    return -19;
  }
  free(str);
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", 0.0, 0.0,
                   0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 16, 0.0, 16, 0.0);
  if (strcmp(
          str,
          "0|       0|00000000|0|       0|0|       0|00000000|               "
          "0|0000000000000000") != 0) {
    free(str);
    return -20;
  }
  free(str);
  // Test %g with trailing zeros
  str = str_format("%.14g", 1.0);
  if (strcmp(str, "1") != 0) {
    free(str);
    return -21;
  }
  free(str);
  // Test %g with big number
  str = str_format("%.14g", 10000000000.0);
  if (strcmp(str, "10000000000") != 0) {
    free(str);
    return -22;
  }
  free(str);
  // Test %g with a precision argument
  str = str_format("%.*g", 4, 10.234);
  if (strcmp(str, "10.23") != 0) {
    free(str);
    return -23;
  }
  free(str);
  // Test length modifiers
  str = str_format("%d %ld %lld %qd %u %lu %llu %qu", 10, 100, 4294967296,
                   4294967296, 10, 100, 4294967296, 4294967296);
  if (strcmp(str,
             "10 100 4294967296 4294967296 10 100 4294967296 4294967296") !=
      0) {
    free(str);
    return -24;
  }
  free(str);
  // Test %.50s with a long string
  str = str_format("%.50s",
                   "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ");
  if (strcmp(str, "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWX") != 0) {
    free(str);
    return -25;
  }
  free(str);
  // Test precision for %x
  str = str_format("%.8x-%.8x-%.2x", 10, 9999999, 9999999);
  if (strcmp(str, "0000000a-0098967f-98967f") != 0) {
    free(str);
    return -26;
  }
  free(str);
  // Test unknown specifier skip
  str = str_format("%I");
  if (strcmp(str, "I") != 0) {
    free(str);
    return -27;
  }
  free(str);
  // Test %s with padding
  const char *s = "Hello";
  str = str_format("[%10s]", s);
  if (strcmp(str, "[     Hello]") != 0) {
    free(str);
    return -28;
  }
  free(str);
  str = str_format("[%-10s]", s);
  if (strcmp(str, "[Hello     ]") != 0) {
    free(str);
    return -29;
  }
  free(str);
  str = str_format("[%*s]", 10, s);
  if (strcmp(str, "[     Hello]") != 0) {
    free(str);
    return -30;
  }
  free(str);
  str = str_format("[%-*s]", 10, s);
  if (strcmp(str, "[Hello     ]") != 0) {
    free(str);
    return -31;
  }
  free(str);
  // Test %p with padding
  str = str_format("%90p", &str);
  if (strlen(str) != 90) {
    free(str);
    return -32;
  }
  free(str);
  // Test sign prepend
  str = str_format("%+08d", 31501);
  if (strcmp(str, "+0031501") != 0) {
    free(str);
    return -33;
  }
  free(str);
  str = str_format("%+08d", -31501);
  if (strcmp(str, "-0031501") != 0) {
    free(str);
    return -34;
  }
  free(str);
  // Test h length modifier (%hd, %hu, %hx, %ho)
  str = str_format("%hd", 42);
  if (strcmp(str, "42") != 0) {
    free(str);
    return -35;
  }
  free(str);
  str = str_format("%hd", -42);
  if (strcmp(str, "-42") != 0) {
    free(str);
    return -36;
  }
  free(str);
  // Truncation: 32768 wraps to -32768 as signed short
  str = str_format("%hd", 32768);
  if (strcmp(str, "-32768") != 0) {
    free(str);
    return -37;
  }
  free(str);
  // Truncation: 65535 is -1 as signed short
  str = str_format("%hd", 65535);
  if (strcmp(str, "-1") != 0) {
    free(str);
    return -38;
  }
  free(str);
  str = str_format("%hu", 65535);
  if (strcmp(str, "65535") != 0) {
    free(str);
    return -39;
  }
  free(str);
  // Truncation: 65536 wraps to 0 as unsigned short
  str = str_format("%hu", 65536);
  if (strcmp(str, "0") != 0) {
    free(str);
    return -40;
  }
  free(str);
  str = str_format("%hx", 0x1234);
  if (strcmp(str, "1234") != 0) {
    free(str);
    return -41;
  }
  free(str);
  // Truncation: upper bits dropped
  str = str_format("%hx", 0x12345);
  if (strcmp(str, "2345") != 0) {
    free(str);
    return -42;
  }
  free(str);
  // Test hh length modifier (%hhd, %hhu, %hhx, %hho)
  str = str_format("%hhd", 127);
  if (strcmp(str, "127") != 0) {
    free(str);
    return -43;
  }
  free(str);
  str = str_format("%hhd", -128);
  if (strcmp(str, "-128") != 0) {
    free(str);
    return -44;
  }
  free(str);
  // Truncation: 128 wraps to -128 as signed char
  str = str_format("%hhd", 128);
  if (strcmp(str, "-128") != 0) {
    free(str);
    return -45;
  }
  free(str);
  // Truncation: 255 is -1 as signed char
  str = str_format("%hhd", 255);
  if (strcmp(str, "-1") != 0) {
    free(str);
    return -46;
  }
  free(str);
  // Truncation: 256 wraps to 0
  str = str_format("%hhd", 256);
  if (strcmp(str, "0") != 0) {
    free(str);
    return -47;
  }
  free(str);
  str = str_format("%hhu", 255);
  if (strcmp(str, "255") != 0) {
    free(str);
    return -48;
  }
  free(str);
  // Truncation: 256 wraps to 0 as unsigned char
  str = str_format("%hhu", 256);
  if (strcmp(str, "0") != 0) {
    free(str);
    return -49;
  }
  free(str);
  // -1 as unsigned char is 255
  str = str_format("%hhu", -1);
  if (strcmp(str, "255") != 0) {
    free(str);
    return -50;
  }
  free(str);
  str = str_format("%hhx", 0xAB);
  if (strcmp(str, "ab") != 0) {
    free(str);
    return -51;
  }
  free(str);
  // Truncation: upper bits dropped
  str = str_format("%hhx", 0x1AB);
  if (strcmp(str, "ab") != 0) {
    free(str);
    return -52;
  }
  free(str);
  // Test %ls (wide string, C locale)
  str = str_format("%ls", L"hello");
  if (strcmp(str, "hello") != 0) {
    free(str);
    return -53;
  }
  free(str);
  // Test %ls with ASCII-only wide string
  str = str_format("%ls", L"foo bar");
  if (strcmp(str, "foo bar") != 0) {
    free(str);
    return -54;
  }
  free(str);
  // Test %ls with empty wide string
  str = str_format("%ls", L"");
  if (strcmp(str, "") != 0) {
    free(str);
    return -55;
  }
  free(str);
  // Test %ls NULL
  str = str_format("%ls", (wchar_t *)NULL);
  if (strcmp(str, "(null)") != 0) {
    free(str);
    return -56;
  }
  free(str);
  // Test %ls embedded in a larger format string
  str = str_format("pre-%ls-post", L"mid");
  if (strcmp(str, "pre-mid-post") != 0) {
    free(str);
    return -57;
  }
  free(str);

  return 0;
}

int test_sscanf() {
  int a, b;
  short c, d;
  float f;
  double lf;
  char str[256], str1[16];
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
  matched = sscanf("FF00", "%2x%2x", &a, &b);
  if (!(matched == 2 && a == 255 && b == 0))
    return -17;
  matched = sscanf("aa", "%10x", &a);
  if (!(matched == 1 && a == 170))
    return -18;
  matched = sscanf("3.14159265359", "%lf", &lf);
  if (!(matched == 1 && lf == 3.14159265359))
    return -19;
  matched = sscanf("hello123", "%[a-z]", str);
  if (!(matched == 1 && strcmp(str, "hello") == 0))
    return -20;
  matched = sscanf("abc123", "%[^0-9]", str);
  if (!(matched == 1 && strcmp(str, "abc") == 0))
    return -21;
  matched = sscanf("-123", "%[-0-9]", str);
  if (!(matched == 1 && strcmp(str, "-123") == 0))
    return -22;
  matched = sscanf("a-b", "%[a-z-]", str);
  if (!(matched == 1 && strcmp(str, "a-b") == 0))
    return -23;
  matched = sscanf("123", "%[^0-9]", str);
  if (matched != 0)
    return -24;
  matched = sscanf("Var_123 =", "%[A-Za-z0-9_]", str);
  if (!(matched == 1 && strcmp(str, "Var_123") == 0))
    return -25;
  matched = sscanf("NAME", "%s", str);
  if (!(matched == 1 && strcmp(str, "NAME") == 0))
    return -26;
  matched = sscanf("   NAME", "%s", str);
  if (!(matched == 1 && strcmp(str, "NAME") == 0))
    return -27;
  matched = sscanf("A B", "%s %s", str, str1);
  if (!(matched == 2 && strcmp(str, "A") == 0 && strcmp(str1, "B") == 0))
    return -28;
  matched = sscanf("numJoints 110\n", " numJoints %d", &a);
  if (!(matched == 1 && a == 110))
    return -29;
  float f1, f2, f3, f4, f5, f6;
  matched = sscanf(
      "	\"origin\"	-1 ( 0 0 0 ) ( -0.7071067095 0 0 )		// ",
      "%s %d ( %f %f %f ) ( %f %f %f )", str, &a, &f1, &f2, &f3, &f4, &f5, &f6);
  if (!(matched == 8 && strcmp(str, "\"origin\"") == 0 && a == -1 && f1 == 0 &&
        fabs(f4 + 0.7071067095) < 1e-10 && f6 == 0))
    return -30;
  // '%g' test cases
  matched = sscanf("123", "%g", &f);
  if (!(matched == 1 && f == 123.0f))
    return -31;
  matched = sscanf("1.23", "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23f) < 1e-5f))
    return -32;
  matched = sscanf("1.23e-4", "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23e-4f) < 1e-8f))
    return -33;
  matched = sscanf("1.23E4", "%g", &f);
  if (!(matched == 1 && fabs(f - 12300.0f) < 1e-5f))
    return -34;
  matched = sscanf("+1.23", "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23f) < 1e-5f))
    return -35;
  matched = sscanf("-1.23", "%g", &f);
  if (!(matched == 1 && fabs(f - -1.23f) < 1e-5f))
    return -36;
  matched = sscanf(".5", "%g", &f);
  if (!(matched == 1 && fabs(f - 0.5f) < 1e-5f))
    return -37;
  matched = sscanf("-.5", "%g", &f);
  if (!(matched == 1 && fabs(f - -0.5f) < 1e-5f))
    return -38;
  matched = sscanf("1e5", "%g", &f);
  if (!(matched == 1 && fabs(f - 100000.0f) < 1e-5f))
    return -39;
  matched = sscanf("1.e5", "%g", &f);
  if (!(matched == 1 && fabs(f - 100000.0f) < 1e-5f))
    return -40;
  matched = sscanf("  1.23", "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23f) < 1e-5f))
    return -41;
  matched = sscanf("+1.23e+4", "%g", &f);
  if (!(matched == 1 && fabs(f - 12300.0f) < 1e-5f))
    return -42;
  matched = sscanf("-1.23e-4", "%g", &f);
  if (!(matched == 1 && fabs(f - -0.000123f) < 1e-8f))
    return -43;
  matched = sscanf("123.", "%g", &f);
  if (!(matched == 1 && f == 123.0f))
    return -44;
  // max_width for %[ specifier
  matched = sscanf("hello", "%3[a-z]", str);
  if (!(matched == 1 && strcmp(str, "hel") == 0))
    return -45;
  matched = sscanf("abcXYZ", "%3[a-z]%3[A-Z]", str, str1);
  if (!(matched == 2 && strcmp(str, "abc") == 0 && strcmp(str1, "XYZ") == 0))
    return -46;
  matched = sscanf("abc,def", "%3[^,],%3[^,]", str, str1);
  if (!(matched == 2 && strcmp(str, "abc") == 0 && strcmp(str1, "def") == 0))
    return -47;
  matched = sscanf("abcdef", "%3[a-z]", str);
  if (!(matched == 1 && strcmp(str, "abc") == 0))
    return -48;
  matched = sscanf("ab", "%5[a-z]", str);
  if (!(matched == 1 && strcmp(str, "ab") == 0))
    return -49;
  // width of 1
  matched = sscanf("abc", "%1[a-z]", str);
  if (!(matched == 1 && strcmp(str, "a") == 0))
    return -50;
  // negated set stopped by width, not by excluded char
  matched = sscanf("abcde", "%3[^X]", str);
  if (!(matched == 1 && strcmp(str, "abc") == 0))
    return -51;
  // negated set stopped by excluded char before width is reached
  matched = sscanf("abXde", "%5[^X]", str);
  if (!(matched == 1 && strcmp(str, "ab") == 0))
    return -52;
  // input length exactly equals width
  matched = sscanf("abc", "%3[a-z]", str);
  if (!(matched == 1 && strcmp(str, "abc") == 0))
    return -53;
  // width limits %[ leaving remainder for next conversion
  matched = sscanf("abcdef", "%3[a-z]%s", str, str1);
  if (!(matched == 2 && strcmp(str, "abc") == 0 && strcmp(str1, "def") == 0))
    return -54;
  // first char not in set with width: no match
  matched = sscanf("123", "%3[a-z]", str);
  if (matched != 0)
    return -55;
  // %hu (unsigned short) edge cases
  unsigned short us, us2;
  matched = sscanf("0", "%hu", &us);
  if (!(matched == 1 && us == 0))
    return -56;
  matched = sscanf("65535", "%hu", &us);
  if (!(matched == 1 && us == 65535))
    return -57;
  // Truncation: 65536 wraps to 0 as unsigned short
  matched = sscanf("65536", "%hu", &us);
  if (!(matched == 1 && us == 0))
    return -58;
  // Truncation: 65537 wraps to 1 as unsigned short
  matched = sscanf("65537", "%hu", &us);
  if (!(matched == 1 && us == 1))
    return -59;
  matched = sscanf("100 200", "%hu %hu", &us, &us2);
  if (!(matched == 2 && us == 100 && us2 == 200))
    return -60;
  // width limits the conversion
  matched = sscanf("12345", "%3hu", &us);
  if (!(matched == 1 && us == 123))
    return -61;
  // %hhu (unsigned char) edge cases
  unsigned char uc, uc2;
  matched = sscanf("0", "%hhu", &uc);
  if (!(matched == 1 && uc == 0))
    return -62;
  matched = sscanf("255", "%hhu", &uc);
  if (!(matched == 1 && uc == 255))
    return -63;
  // Truncation: 256 wraps to 0 as unsigned char
  matched = sscanf("256", "%hhu", &uc);
  if (!(matched == 1 && uc == 0))
    return -64;
  // Truncation: 257 wraps to 1 as unsigned char
  matched = sscanf("257", "%hhu", &uc);
  if (!(matched == 1 && uc == 1))
    return -65;
  matched = sscanf("10 20", "%hhu %hhu", &uc, &uc2);
  if (!(matched == 2 && uc == 10 && uc2 == 20))
    return -66;
  // width limits the conversion
  matched = sscanf("12345", "%2hhu", &uc);
  if (!(matched == 1 && uc == 12))
    return -67;
  // Overflow above UINT_MAX: per C semantics, the input is parsed as
  // a wide unsigned and only the low bits are stored, so 0x100000000
  // gives 0 in both u16 and u8.
  matched = sscanf("4294967296", "%hu", &us);
  if (!(matched == 1 && us == 0))
    return -68;
  matched = sscanf("4294967296", "%hhu", &uc);
  if (!(matched == 1 && uc == 0))
    return -69;
  // %hx (unsigned short, hex) truncation
  matched = sscanf("ffff", "%hx", &us);
  if (!(matched == 1 && us == 0xFFFF))
    return -70;
  // Truncation: 0x10000 wraps to 0 as unsigned short
  matched = sscanf("10000", "%hx", &us);
  if (!(matched == 1 && us == 0))
    return -71;
  // Truncation: 0x10001 wraps to 1 as unsigned short
  matched = sscanf("10001", "%hx", &us);
  if (!(matched == 1 && us == 1))
    return -72;
  // %hhx (unsigned char, hex) truncation
  matched = sscanf("ff", "%hhx", &uc);
  if (!(matched == 1 && uc == 0xFF))
    return -73;
  // Truncation: 0x100 wraps to 0 as unsigned char
  matched = sscanf("100", "%hhx", &uc);
  if (!(matched == 1 && uc == 0))
    return -74;
  // Truncation: 0x101 wraps to 1 as unsigned char
  matched = sscanf("101", "%hhx", &uc);
  if (!(matched == 1 && uc == 1))
    return -75;
  // %hd (signed short) edge cases
  short ss, ss2;
  matched = sscanf("0", "%hd", &ss);
  if (!(matched == 1 && ss == 0))
    return -76;
  matched = sscanf("32767", "%hd", &ss);
  if (!(matched == 1 && ss == 32767))
    return -77;
  matched = sscanf("-32768", "%hd", &ss);
  if (!(matched == 1 && ss == -32768))
    return -78;
  // Truncation: 32768 wraps to -32768 as signed short
  matched = sscanf("32768", "%hd", &ss);
  if (!(matched == 1 && ss == -32768))
    return -79;
  // Truncation: -32769 wraps to 32767 as signed short
  matched = sscanf("-32769", "%hd", &ss);
  if (!(matched == 1 && ss == 32767))
    return -80;
  matched = sscanf("-100 200", "%hd %hd", &ss, &ss2);
  if (!(matched == 2 && ss == -100 && ss2 == 200))
    return -81;
  // width limits the conversion
  matched = sscanf("12345", "%3hd", &ss);
  if (!(matched == 1 && ss == 123))
    return -82;
  // width counts the sign character
  matched = sscanf("-12345", "%4hd", &ss);
  if (!(matched == 1 && ss == -123))
    return -83;
  // %hhd (signed char) edge cases
  signed char sc, sc2;
  matched = sscanf("0", "%hhd", &sc);
  if (!(matched == 1 && sc == 0))
    return -84;
  matched = sscanf("127", "%hhd", &sc);
  if (!(matched == 1 && sc == 127))
    return -85;
  matched = sscanf("-128", "%hhd", &sc);
  if (!(matched == 1 && sc == -128))
    return -86;
  // Truncation: 128 wraps to -128 as signed char
  matched = sscanf("128", "%hhd", &sc);
  if (!(matched == 1 && sc == -128))
    return -87;
  // Truncation: -129 wraps to 127 as signed char
  matched = sscanf("-129", "%hhd", &sc);
  if (!(matched == 1 && sc == 127))
    return -88;
  matched = sscanf("-10 20", "%hhd %hhd", &sc, &sc2);
  if (!(matched == 2 && sc == -10 && sc2 == 20))
    return -89;
  // width limits the conversion
  matched = sscanf("12345", "%2hhd", &sc);
  if (!(matched == 1 && sc == 12))
    return -90;
  // width counts the sign character
  matched = sscanf("-12345", "%3hhd", &sc);
  if (!(matched == 1 && sc == -12))
    return -91;
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

int test_realloc() {
  void *ptr = realloc(NULL, 32);
  memmove(ptr, "abcd", 4);
  ptr = realloc(ptr, 64);
  int res = memcmp(ptr, "abcd", 4);
  free(ptr);
  return res == 0 ? 0 : -1;
}

int test_valloc() {
  void *ptr = valloc(1);
  // Assume at least 4Kb page size alignment
  if (((uintptr_t)ptr & 0xFFF) != 0) {
    return -1;
  }
  if (ptr == NULL)
    return -2;
  ptr = realloc(ptr, 16);
  if (ptr == NULL)
    return -3;
  free(ptr);
  return 0;
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
  text = "12345";
  if (strtoul(text, &endptr, 10) != 12345UL || endptr != text + 5) {
    return -2;
  }
  text = "123abc";
  if (strtoul(text, &endptr, 10) != 123UL || endptr != text + 3) {
    return -3;
  }
  text = "abc";
  if (strtoul(text, &endptr, 10) != 0UL || endptr != text) {
    return -4;
  }
  text = "-1";
  if (strtoul(text, &endptr, 10) != (unsigned long)-1 || endptr != text + 2) {
    return -5;
  }
  text = "Ff";
  if (strtoul(text, &endptr, 16) != 255UL || endptr != text + 2) {
    return -6;
  }
  text = "   +42abc";
  if (strtoul(text, &endptr, 10) != 42UL || endptr != text + 6) {
    return -7;
  }
#ifndef DEFINE_ME_WHEN_BUILDING_ON_MACOS
  // Test for overflow. "4294967296" is ULONG_MAX + 1 on a 32-bit system.
  text = "4294967296";
  if (strtoul(text, &endptr, 10) != 4294967295 || endptr != text + 10) {
    return -8;
  }
#endif
  text = "4294967295";
  if (strtoul(text, &endptr, 10) != 4294967295 || endptr != text + 10) {
    return -9;
  }
  text = "15";
  if (strtoul(text, &endptr, 0) != 15UL || endptr != text + 2) {
    return -10;
  }
  text = "017"; // octal: 1*8 + 7 = 15
  if (strtoul(text, &endptr, 0) != 15UL || endptr != text + 3) {
    return -11;
  }
  text = "0x0F";
  if (strtoul(text, &endptr, 0) != 15UL || endptr != text + 4) {
    return -12;
  }
  text = "";
  if (strtoul(text, &endptr, 10) != 0UL || endptr != text) {
    return -13;
  }
  text = "   ";
  if (strtoul(text, &endptr, 10) != 0UL || endptr != text) {
    return -14;
  }
  text = "1101"; // binary: 8 + 4 + 1 = 13
  if (strtoul(text, &endptr, 2) != 13UL || endptr != text + 4) {
    return -15;
  }
  text = "zZ"; // base 36: 35*36 + 35 = 1295
  if (strtoul(text, &endptr, 36) != 1295UL || endptr != text + 2) {
    return -16;
  }
  text = "77"; // octal: 7*8 + 7 = 63
  if (strtoul(text, &endptr, 8) != 63UL || endptr != text + 2) {
    return -17;
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

  // Sem @ 0
  if (sem_trywait(semaphore) == -1) {
    return -1;
  }

  // Sem still @ 0, should not lock
  if (sem_trywait(semaphore) == 0) {
    return -1;
  }

  // Sem @ 1, should be able to relock
  sem_post(semaphore);
  if (sem_trywait(semaphore) == -1) {
    return -1;
  }

  sem_close(semaphore);
  sem_unlink("sem_test");
  return 0;
}

sem_t *mt_semaphore;

void mtsem_thread() {
  sem_wait(mt_semaphore);
  sem_post(mt_semaphore);
}

int test_mtsem() {
  mt_semaphore = sem_open("mtsem_test", O_CREAT, 0644, 0);
  if (mt_semaphore == SEM_FAILED) {
    printf("Error opening semaphore\n");
    return -1;
  }

  pthread_t *my_thread = (pthread_t *)malloc(sizeof(pthread_t));
  pthread_create(my_thread, NULL, (void *)mtsem_thread, NULL);

  pthread_t *my_thread2 = (pthread_t *)malloc(sizeof(pthread_t));
  pthread_create(my_thread2, NULL, (void *)mtsem_thread, NULL);

  usleep(1);
  usleep(1);

  sem_post(mt_semaphore);
  pthread_join(*my_thread, NULL);
  pthread_join(*my_thread2, NULL);
  return 0;
}

sem_t *thread_suspend_semaphore;
sem_t *thread_suspend_ready;
int thread_suspend_flag = 0;

void *thread_suspend_thread_func(void *arg) {
  sem_post(thread_suspend_ready);
  sem_wait(thread_suspend_semaphore);
  thread_suspend_flag = 1;
  return NULL;
}

int test_thread_suspend_resume() {
  thread_suspend_flag = 0;

  thread_suspend_semaphore = sem_open("thread_suspend_test", O_CREAT, 0644, 0);
  if (thread_suspend_semaphore == SEM_FAILED) {
    printf("Error opening semaphore\n");
    return -1;
  }
  thread_suspend_ready = sem_open("thread_suspend_ready", O_CREAT, 0644, 0);
  if (thread_suspend_ready == SEM_FAILED) {
    sem_close(thread_suspend_semaphore);
    sem_unlink("thread_suspend_test");
    printf("Error opening ready semaphore\n");
    return -2;
  }

  pthread_t thr;
  if (pthread_create(&thr, NULL, thread_suspend_thread_func, NULL) != 0)
    return -3;

  // Wait until the worker has reached sem_wait().
  sem_wait(thread_suspend_ready);

  mach_port_t thr_port = pthread_mach_thread_np(thr);
  if (thread_suspend(thr_port) != KERN_SUCCESS)
    return -4;

  // Check that run_state reports TH_STATE_WAITING while suspended.
  thread_basic_info_data_t thr_info;
  mach_msg_type_number_t info_count = THREAD_BASIC_INFO_COUNT;
  if (thread_info(thr_port, THREAD_BASIC_INFO, (thread_info_t)&thr_info,
                  &info_count) != KERN_SUCCESS)
    return -5;
  if (thr_info.run_state != TH_STATE_WAITING)
    return -6;

  // Post the semaphore while the thread is suspended - it should not wake up.
  sem_post(thread_suspend_semaphore);
  sched_yield();

  if (thread_suspend_flag != 0)
    return -7;

  if (thread_resume(thr_port) != KERN_SUCCESS)
    return -8;

  pthread_join(thr, NULL);

  if (thread_suspend_flag != 1)
    return -9;

  sem_close(thread_suspend_semaphore);
  sem_unlink("thread_suspend_test");
  sem_close(thread_suspend_ready);
  sem_unlink("thread_suspend_ready");
  return 0;
}

int done = 0, done2 = 0;
pthread_mutex_t m;
pthread_cond_t c, c2;

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

void *child2(void *arg) {
  pthread_mutex_lock(&m);
  while (done2 == 0) {
    pthread_cond_wait(&c2, &m);
  }
  pthread_mutex_unlock(&m);
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

  if (done != 1)
    return -1;

  // Should wake up all threads
  int result;
  pthread_t p1, p2, p3;
  result = pthread_cond_init(&c2, NULL);
  if (result != 0)
    return -2;
  pthread_create(&p1, NULL, child2, NULL);
  pthread_create(&p2, NULL, child2, NULL);
  pthread_create(&p3, NULL, child2, NULL);
  usleep(100);
  result = pthread_mutex_lock(&m);
  if (result != 0)
    return -3;
  done2 = 1;
  result = pthread_cond_broadcast(&c2);
  if (result != 0)
    return -4;
  result = pthread_mutex_unlock(&m);
  if (result != 0)
    return -5;
  pthread_join(p1, NULL);
  pthread_join(p2, NULL);
  pthread_join(p3, NULL);

  return 0;
}

pthread_mutex_t m_static = PTHREAD_MUTEX_INITIALIZER;
pthread_cond_t c_static = PTHREAD_COND_INITIALIZER,
               c2_static = PTHREAD_COND_INITIALIZER;

void thr_exit_static() {
  pthread_mutex_lock(&m_static);
  done = 1;
  pthread_cond_signal(&c_static);
  pthread_mutex_unlock(&m_static);
}

void *child_static(void *arg) {
  thr_exit_static();
  return NULL;
}

void *child2_static(void *arg) {
  pthread_mutex_lock(&m_static);
  while (done2 == 0) {
    pthread_cond_wait(&c2_static, &m_static);
  }
  pthread_mutex_unlock(&m_static);
  return NULL;
}

void thr_join_static() {
  pthread_mutex_lock(&m_static);
  while (done == 0) {
    pthread_cond_wait(&c_static, &m_static);
  }
  pthread_mutex_unlock(&m_static);
}

int test_cond_var_static() {
  pthread_t p;
  done = 0;
  done2 = 0;

  // We test that statically allocated cond vars and mutexes work
  // by using them without calling pthread_mutex_init and pthread_cond_init

  pthread_create(&p, NULL, child_static, NULL);
  thr_join_static();

  if (done != 1)
    return -1;

  // Should wake up all threads
  int result;
  pthread_t p1, p2, p3;
  pthread_create(&p1, NULL, child2_static, NULL);
  pthread_create(&p2, NULL, child2_static, NULL);
  pthread_create(&p3, NULL, child2_static, NULL);
  usleep(100);
  result = pthread_mutex_lock(&m_static);
  if (result != 0)
    return -2;
  done2 = 1;
  result = pthread_cond_broadcast(&c2_static);
  if (result != 0)
    return -3;
  result = pthread_mutex_unlock(&m_static);
  if (result != 0)
    return -4;
  pthread_join(p1, NULL);
  pthread_join(p2, NULL);
  pthread_join(p3, NULL);

  return 0;
}

// === pthread_cond_timedwait tests ===

struct timedwait_signaler_args {
  pthread_mutex_t *mu;
  pthread_cond_t *cv;
};

void *timedwait_signaler(void *arg) {
  struct timedwait_signaler_args *a = arg;
  usleep(20000); // 20ms - well before the 500ms deadline
  pthread_mutex_lock(a->mu);
  pthread_cond_signal(a->cv);
  pthread_mutex_unlock(a->mu);
  return NULL;
}

// Test : signal arrives before deadline - should return 0, not ETIMEDOUT
int test_cond_timedwait_signaled_before_timeout() {
  pthread_mutex_t mu;
  pthread_cond_t cv;
  if (pthread_mutex_init(&mu, NULL) != 0)
    return -1;
  if (pthread_cond_init(&cv, NULL) != 0)
    return -2;

  struct timedwait_signaler_args args = {&mu, &cv};
  pthread_t p;
  if (pthread_create(&p, NULL, timedwait_signaler, &args) != 0)
    return -3;

  struct timeval tv;
  gettimeofday(&tv, NULL);
  // Generous 500ms deadline
  struct timespec ts = {.tv_sec = tv.tv_sec,
                        .tv_nsec = tv.tv_usec * 1000 + 500000000};
  if (ts.tv_nsec >= 1000000000) {
    ts.tv_sec += 1;
    ts.tv_nsec -= 1000000000;
  }

  pthread_mutex_lock(&mu);
  int result = pthread_cond_timedwait(&cv, &mu, &ts);
  pthread_mutex_unlock(&mu);

  pthread_join(p, NULL);
  pthread_cond_destroy(&cv);
  pthread_mutex_destroy(&mu);

  if (result != 0)
    return -4;
  return 0;
}

// Test : deadline already in the past - should return ETIMEDOUT immediately
int test_cond_timedwait_past_deadline() {
  pthread_mutex_t mu;
  pthread_cond_t cv;
  if (pthread_mutex_init(&mu, NULL) != 0)
    return -1;
  if (pthread_cond_init(&cv, NULL) != 0)
    return -2;

  // Use a timestamp far in the past
  struct timespec ts = {.tv_sec = 1, .tv_nsec = 0};

  pthread_mutex_lock(&mu);
  int result = pthread_cond_timedwait(&cv, &mu, &ts);
  pthread_mutex_unlock(&mu);

  pthread_cond_destroy(&cv);
  pthread_mutex_destroy(&mu);

  if (result != ETIMEDOUT)
    return -3;
  return 0;
}

struct timedwait_broadcast_args {
  pthread_mutex_t *mu;
  pthread_cond_t *cv;
  int ready;
};

void *timedwait_broadcast_waiter(void *arg) {
  struct timedwait_broadcast_args *a = arg;
  struct timeval tv;
  gettimeofday(&tv, NULL);
  struct timespec ts = {.tv_sec = tv.tv_sec,
                        .tv_nsec = tv.tv_usec * 1000 + 500000000};
  if (ts.tv_nsec >= 1000000000) {
    ts.tv_sec += 1;
    ts.tv_nsec -= 1000000000;
  }
  pthread_mutex_lock(a->mu);
  while (a->ready == 0)
    pthread_cond_timedwait(a->cv, a->mu, &ts);
  pthread_mutex_unlock(a->mu);
  return NULL;
}

// Test: broadcast wakes all timedwait threads
int test_cond_timedwait_broadcast() {
  pthread_mutex_t mu;
  pthread_cond_t cv;
  if (pthread_mutex_init(&mu, NULL) != 0)
    return -1;
  if (pthread_cond_init(&cv, NULL) != 0)
    return -2;

  struct timedwait_broadcast_args args = {&mu, &cv, 0};
  pthread_t p1, p2, p3;
  if (pthread_create(&p1, NULL, timedwait_broadcast_waiter, &args) != 0)
    return -3;
  if (pthread_create(&p2, NULL, timedwait_broadcast_waiter, &args) != 0)
    return -4;
  if (pthread_create(&p3, NULL, timedwait_broadcast_waiter, &args) != 0)
    return -5;

  usleep(50000); // let all three threads reach their waits
  pthread_mutex_lock(&mu);
  args.ready = 1;
  int result = pthread_cond_broadcast(&cv);
  pthread_mutex_unlock(&mu);

  if (result != 0)
    return -6;

  pthread_join(p1, NULL);
  pthread_join(p2, NULL);
  pthread_join(p3, NULL);

  pthread_cond_destroy(&cv);
  pthread_mutex_destroy(&mu);
  return 0;
}

// Test: timed-out state on a cond must not persist across calls. After one
// thread times out, a later signaled timedwait on the same cond must
// return 0, not ETIMEDOUT.
int test_cond_timedwait_flag_not_sticky() {
  pthread_mutex_t mu;
  pthread_cond_t cv;
  if (pthread_mutex_init(&mu, NULL) != 0)
    return -1;
  if (pthread_cond_init(&cv, NULL) != 0)
    return -2;

  // First call: force an immediate timeout.
  struct timespec past = {.tv_sec = 1, .tv_nsec = 0};
  pthread_mutex_lock(&mu);
  int first = pthread_cond_timedwait(&cv, &mu, &past);
  pthread_mutex_unlock(&mu);
  if (first != ETIMEDOUT)
    return -3;

  // Second call on the same cond: should be signaled and return 0.
  struct timedwait_signaler_args args = {&mu, &cv};
  pthread_t p;
  if (pthread_create(&p, NULL, timedwait_signaler, &args) != 0)
    return -4;

  struct timeval tv;
  gettimeofday(&tv, NULL);
  struct timespec ts = {.tv_sec = tv.tv_sec,
                        .tv_nsec = tv.tv_usec * 1000 + 500000000};
  if (ts.tv_nsec >= 1000000000) {
    ts.tv_sec += 1;
    ts.tv_nsec -= 1000000000;
  }

  pthread_mutex_lock(&mu);
  int second = pthread_cond_timedwait(&cv, &mu, &ts);
  pthread_mutex_unlock(&mu);

  pthread_join(p, NULL);
  pthread_cond_destroy(&cv);
  pthread_mutex_destroy(&mu);

  if (second != 0)
    return -5;
  return 0;
}

struct timedwait_sibling_args {
  pthread_mutex_t *mu;
  pthread_cond_t *cv;
  long sec_offset;
  long ns_offset; // must be < 1000000000
  int result;
};

void *timedwait_sibling_sleeper(void *arg) {
  (void)arg;
  usleep(200000);
  return NULL;
}

void *timedwait_sibling_waiter(void *arg) {
  struct timedwait_sibling_args *a = arg;
  struct timeval tv;
  gettimeofday(&tv, NULL);
  struct timespec ts = {.tv_sec = tv.tv_sec + a->sec_offset,
                        .tv_nsec = tv.tv_usec * 1000 + a->ns_offset};
  if (ts.tv_nsec >= 1000000000) {
    ts.tv_sec += 1;
    ts.tv_nsec -= 1000000000;
  }
  pthread_mutex_lock(a->mu);
  a->result = pthread_cond_timedwait(a->cv, a->mu, &ts);
  pthread_mutex_unlock(a->mu);
  return NULL;
}

// Test: when one waiter times out, other waiters on the same cond must
// still be reachable by a later signal - i.e. a timeout on one thread
// must not drop sibling waiters from the queue.
int test_cond_timedwait_sibling_not_dropped() {
  pthread_mutex_t mu;
  pthread_cond_t cv;
  if (pthread_mutex_init(&mu, NULL) != 0)
    return -1;
  if (pthread_cond_init(&cv, NULL) != 0)
    return -2;

  struct timedwait_sibling_args long_args = {&mu, &cv, 3, 0, -999};
  struct timedwait_sibling_args short_args = {&mu, &cv, 0, 100000000, -999};
  pthread_t p_long, p_short, p_sleeper;
  if (pthread_create(&p_long, NULL, timedwait_sibling_waiter, &long_args) != 0)
    return -3;
  usleep(20000); // let the long-deadline waiter enter the wait first
  if (pthread_create(&p_short, NULL, timedwait_sibling_waiter, &short_args) !=
      0)
    return -4;
  // A thread unrelated to the cond var - keeps the scheduler ticking while
  // the waiters are blocked on Condition, and verifies that sibling fallout
  // from the timeout doesn't leak to unrelated threads.
  if (pthread_create(&p_sleeper, NULL, timedwait_sibling_sleeper, NULL) != 0)
    return -8;

  // Join the short waiter first. This forces the scheduler to actually
  // process short's timeout before we signal, so the bug path (which
  // clears the entire waiting queue on timeout) has a chance to fire.
  pthread_join(p_short, NULL);
  if (short_args.result != ETIMEDOUT)
    return -5;

  // Now signal. The long waiter must still be reachable.
  pthread_mutex_lock(&mu);
  int res = pthread_cond_signal(&cv);
  pthread_mutex_unlock(&mu);
  if (res != 0)
    return -6;

  pthread_join(p_long, NULL);
  pthread_join(p_sleeper, NULL);
  pthread_cond_destroy(&cv);
  pthread_mutex_destroy(&mu);

  if (long_args.result != 0)
    return -7;
  return 0;
}

// === end pthread_cond_timedwait tests ===

pthread_mutex_t normal_mutex;
int normal_unlock_res = -1;

void *normal_unlocker(void *arg) {
  normal_unlock_res = pthread_mutex_unlock(&normal_mutex);
  return NULL;
}

int test_pthread_mutex_normal() {
  pthread_mutexattr_t attr;
  if (pthread_mutexattr_init(&attr) != 0)
    return -1;
  if (pthread_mutexattr_settype(&attr, PTHREAD_MUTEX_NORMAL) != 0)
    return -2;
  if (pthread_mutex_init(&normal_mutex, &attr) != 0)
    return -3;
  if (pthread_mutexattr_destroy(&attr) != 0)
    return -4;

  if (pthread_mutex_lock(&normal_mutex) != 0)
    return -5;

  pthread_t p;
  if (pthread_create(&p, NULL, normal_unlocker, NULL) != 0)
    return -6;
  if (pthread_join(p, NULL) != 0)
    return -7;

  if (pthread_mutex_destroy(&normal_mutex) != 0)
    return -8;

  if (normal_unlock_res != 0)
    return -9;

  return 0;
}

pthread_mutex_t recursive_mutex;
int recursive_trylock_res = -1;

void *recursive_trylocker(void *arg) {
  recursive_trylock_res = pthread_mutex_trylock(&recursive_mutex);
  return NULL;
}

int test_pthread_mutex_recursive_trylock() {
  pthread_mutexattr_t attr;
  if (pthread_mutexattr_init(&attr) != 0)
    return -1;
  if (pthread_mutexattr_settype(&attr, PTHREAD_MUTEX_RECURSIVE) != 0)
    return -2;
  if (pthread_mutex_init(&recursive_mutex, &attr) != 0)
    return -3;
  if (pthread_mutexattr_destroy(&attr) != 0)
    return -4;

  if (pthread_mutex_trylock(&recursive_mutex) != 0)
    return -5;

  if (pthread_mutex_trylock(&recursive_mutex) != 0)
    return -6;

  pthread_t p;
  if (pthread_create(&p, NULL, recursive_trylocker, NULL) != 0)
    return -7;
  if (pthread_join(p, NULL) != 0)
    return -8;

  if (recursive_trylock_res != EBUSY)
    return -9;

  if (pthread_mutex_unlock(&recursive_mutex) != 0)
    return -10;
  if (pthread_mutex_unlock(&recursive_mutex) != 0)
    return -11;

  if (pthread_mutex_destroy(&recursive_mutex) != 0)
    return -12;

  return 0;
}

int second_thread_thread_size_res = -1;

void *second_thread(void *arg) {
  size_t stack_size = pthread_get_stacksize_np(pthread_self());
  if (stack_size == 512 * 1024) {
    second_thread_thread_size_res = 0;
  }
  return NULL;
}

int test_pthread_get_stacksize_np() {
  size_t stack_size = pthread_get_stacksize_np(pthread_self());
  if (stack_size != 1024 * 1024 - getpagesize()) {
    return -1;
  }

  pthread_t p;
  if (pthread_create(&p, NULL, second_thread, NULL) != 0)
    return -2;
  if (pthread_join(p, NULL) != 0)
    return -3;
  if (second_thread_thread_size_res != 0)
    return -4;

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
  locale = setlocale(LC_NUMERIC, "POSIX");
  if (strcmp(locale, "POSIX") != 0) {
    return 2;
  }

  // Test if other categories are unaffected
  locale = setlocale(LC_TIME, NULL);
  if (strcmp(locale, "C") != 0) {
    return 3;
  }

  // Set C locale back for numeric
  locale = setlocale(LC_NUMERIC, "C");
  if (strcmp(locale, "C") != 0) {
    return 4;
  }

  return 0;
}

const int PATH_BUF_SIZE = 256;
// static array for path: not great, not terrible
char path[PATH_BUF_SIZE];

const char *path_test_app() {
#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
  // assume project dir as cwd
  return "./tests/TestApp.app";
#else
  bzero(path, PATH_BUF_SIZE);
  CFBundleRef mainBundle = CFBundleGetMainBundle();
  CFURLRef bundleURL = CFBundleCopyBundleURL(mainBundle);
  CFURLGetFileSystemRepresentation(bundleURL,
                                   true, // Resolve against base (absolute path)
                                   (UInt8 *)path, // Output buffer
                                   PATH_BUF_SIZE  // Buffer size
  );
  CFRelease(bundleURL);
  return path;
#endif
}

int test_dirent() {
  struct dirent *dp;
  DIR *dirp = opendir(path_test_app());
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
  int n = scandir(path_test_app(), &namelist, NULL, NULL);
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

int test_read_directory_as_fd() {
  FILE *dir_stream = fopen(path_test_app(), "r");
  if (dir_stream == NULL) {
    return -1;
  }
  char buffer[1024];
  size_t bytes_read = fread(buffer, 1, 4, dir_stream);
  if (bytes_read != 0) {
    return -2;
  }
  if (errno != EISDIR) {
    return -3;
  }
  fclose(dir_stream);
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
  if (chdir(path_test_app()))
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

int test_ungetc() {
  FILE *file = fopen("test_ungetc", "r");
  if (file == NULL) {
    return -1;
  }
  char c = getc(file);
  if (c != 'a') {
    fclose(file);
    return -2;
  }
  // ungetc with _wrong_ char
  c = ungetc('b', file);
  if (c != 'b') {
    fclose(file);
    return -3;
  }
  char buf[4];
  memset(buf, '\0', 4);
  size_t read = fread(buf, 1, 3, file);
  fclose(file);
  if (read != 3) {
    return -4;
  }
  if (strcmp(buf, "baa") != 0) {
    return -5;
  }
  return 0;
}

int test_fscanf() {
  char str[256];
  int a;
  float f;
  FILE *file = fopen("test_fscanf", "r");
  if (file == NULL) {
    return -1;
  }
  int matched = fscanf(file, "%s", str);
  if (!(matched == 1 && strcmp(str, "no_spaces_line") == 0)) {
    return -2;
  }
  matched = fscanf(file, "%s %d", str, &a);
  if (!(matched == 2 && strcmp(str, "one") == 0 && a == -100)) {
    return -3;
  }
  matched = fscanf(file, "%s", str);
  if (!(matched == 1 && strcmp(str, "string") == 0)) {
    return -4;
  }
  matched = fscanf(file, "%f", &f);
  if (!(matched == 1 && fabs(f - 3.14) < 0.001)) {
    return -5;
  }
  matched = fscanf(file, "%s", str);
  if (matched != -1) { // EOF
    return -6;
  }
  fclose(file);
  return 0;
}

// Below tests are on par with test_sscanf(),
// but reading data from a file instead.
// Please update those as well if you add new
// test cases to test_sscanf()
int test_fscanf_new() {
  FILE *file = fopen("test_fscanf_new", "r");
  if (!file)
    return -1;

#define SKIP_LINE(f)                                                           \
  do {                                                                         \
    int ch;                                                                    \
    while ((ch = fgetc(f)) != '\n' && ch != -1)                                \
      ;                                                                        \
  } while (0)

  int a, b, matched;
  short c, d;
  float f, f1, f2, f3, f4, f5, f6;
  double lf;
  char str[256], str1[4];

  matched = fscanf(file, "%d.%d", &a, &b);
  if (!(matched == 2 && a == 1 && b == 23))
    return -2;
  SKIP_LINE(file);

  matched = fscanf(file, "abc%d.%d", &a, &b);
  if (!(matched == 2 && a == 111 && b == 42))
    return -3;
  SKIP_LINE(file);

  matched = fscanf(file, "%d.%d", &a, &b);
  if (matched != 0)
    return -4;
  SKIP_LINE(file);

  matched = fscanf(file, "%[^,],%d", str, &b);
  if (!(matched == 2 && strcmp(str, "abc") == 0 && b == 8))
    return -5;
  SKIP_LINE(file);

  matched = fscanf(file, "%hi,%i", &c, &a);
  if (!(matched == 2 && c == 9 && a == 10))
    return -6;
  SKIP_LINE(file);

  matched = fscanf(file, "%d", &a);
  if (matched != 0)
    return -7;
  SKIP_LINE(file);

  matched = fscanf(file, "%d %d", &a, &b);
  if (!(matched == 2 && a == 10 && b == -10))
    return -8;
  SKIP_LINE(file);

  matched = fscanf(file, "%hd %hd", &c, &d);
  if (!(matched == 2 && c == 10 && d == -10))
    return -9;
  SKIP_LINE(file);

  matched = fscanf(file, "%d %d", &a, &b);
  if (!(matched == 1 && a == 3000))
    return -10;
  SKIP_LINE(file);

  matched = fscanf(file, "%08x", &a);
  if (!(matched == 1 && a == 16711680))
    return -11;
  SKIP_LINE(file);

  matched = fscanf(file, "%s %f", str, &f);
  if (!(matched == 2 && strcmp(str, "ABC") == 0 && f == 1.0f))
    return -12;
  SKIP_LINE(file);

  matched = fscanf(file, "%s\t%f", str, &f);
  if (!(matched == 2 && strcmp(str, "ABC") == 0 && f == 1.0f))
    return -13;
  SKIP_LINE(file);

  matched = fscanf(file, "%s %f", str, &f);
  if (!(matched == 2 && strcmp(str, "MAX") == 0 && f == 48.0f))
    return -14;
  SKIP_LINE(file);

  matched = fscanf(file, "%i", &a);
  if (!(matched == 1 && a == 9))
    return -15;
  SKIP_LINE(file);

  matched = fscanf(file, "%i", &a);
  if (!(matched == 1 && a == 0))
    return -16;
  SKIP_LINE(file);

  matched = fscanf(file, "%2x%2x", &a, &b);
  if (!(matched == 2 && a == 0xFF && b == 0x00))
    return -17;
  SKIP_LINE(file);

  matched = fscanf(file, "%10x", &a);
  if (!(matched == 1 && a == 0xAA))
    return -18;
  SKIP_LINE(file);

  matched = fscanf(file, "%lf", &lf);
  if (!(matched == 1 && lf == 3.14159265359))
    return -19;
  SKIP_LINE(file);

  matched = fscanf(file, "%[a-z]", str);
  if (!(matched == 1 && strcmp(str, "hello") == 0))
    return -20;
  SKIP_LINE(file);

  matched = fscanf(file, "%[^0-9]", str);
  if (!(matched == 1 && strcmp(str, "abc") == 0))
    return -21;
  SKIP_LINE(file);

  matched = fscanf(file, "%[-0-9]", str);
  if (!(matched == 1 && strcmp(str, "-123") == 0))
    return -22;
  SKIP_LINE(file);

  matched = fscanf(file, "%[a-z-]", str);
  if (!(matched == 1 && strcmp(str, "a-b") == 0))
    return -23;
  SKIP_LINE(file);

  matched = fscanf(file, "%[^0-9]", str);
  if (matched != 0)
    return -24;
  SKIP_LINE(file);

  matched = fscanf(file, "%[A-Za-z0-9_]", str);
  if (!(matched == 1 && strcmp(str, "Var_123") == 0))
    return -25;
  SKIP_LINE(file);

  matched = fscanf(file, "%s", str);
  if (!(matched == 1 && strcmp(str, "NAME") == 0))
    return -26;
  SKIP_LINE(file);

  matched = fscanf(file, "%s", str);
  if (!(matched == 1 && strcmp(str, "NAME") == 0))
    return -27;
  SKIP_LINE(file);

  matched = fscanf(file, "%s %s", str, str1);
  if (!(matched == 2 && strcmp(str, "A") == 0 && strcmp(str1, "B") == 0))
    return -28;
  SKIP_LINE(file);

  matched = fscanf(file, " numJoints %d", &a);
  if (!(matched == 1 && a == 110))
    return -29;
  SKIP_LINE(file);

  matched = fscanf(file, " %s %d ( %f %f %f ) ( %f %f %f )", str, &a, &f1, &f2,
                   &f3, &f4, &f5, &f6);
  if (!(matched == 8 && strcmp(str, "\"origin\"") == 0 && a == -1 &&
        f1 == 0.0f && fabs(f4 + 0.7071067095f) < 1e-10f && f6 == 0.0f))
    return -30;
  SKIP_LINE(file);

  // '%g' test cases
  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && f == 123.0f))
    return -31;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23f) < 1e-5f))
    return -32;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23e-4f) < 1e-8f))
    return -33;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 12300.0f) < 1e-5f))
    return -34;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23f) < 1e-5f))
    return -35;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - -1.23f) < 1e-5f))
    return -36;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 0.5f) < 1e-5f))
    return -37;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - -0.5f) < 1e-5f))
    return -38;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 100000.0f) < 1e-5f))
    return -39;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 100000.0f) < 1e-5f))
    return -40;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 1.23f) < 1e-5f))
    return -41;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - 12300.0f) < 1e-5f))
    return -42;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && fabs(f - -0.000123f) < 1e-8f))
    return -43;
  SKIP_LINE(file);

  matched = fscanf(file, "%g", &f);
  if (!(matched == 1 && f == 123.0f))
    return -44;
  SKIP_LINE(file);

  // %hu (unsigned short) edge cases
  unsigned short us, us2;
  matched = fscanf(file, "%hu", &us);
  if (!(matched == 1 && us == 0))
    return -45;
  SKIP_LINE(file);

  matched = fscanf(file, "%hu", &us);
  if (!(matched == 1 && us == 65535))
    return -46;
  SKIP_LINE(file);

  // Truncation: 65536 wraps to 0 as unsigned short
  matched = fscanf(file, "%hu", &us);
  if (!(matched == 1 && us == 0))
    return -47;
  SKIP_LINE(file);

  // Truncation: 65537 wraps to 1 as unsigned short
  matched = fscanf(file, "%hu", &us);
  if (!(matched == 1 && us == 1))
    return -48;
  SKIP_LINE(file);

  matched = fscanf(file, "%hu %hu", &us, &us2);
  if (!(matched == 2 && us == 100 && us2 == 200))
    return -49;
  SKIP_LINE(file);

  // width limits the conversion
  matched = fscanf(file, "%3hu", &us);
  if (!(matched == 1 && us == 123))
    return -50;
  SKIP_LINE(file);

  // %hhu (unsigned char) edge cases
  unsigned char uc, uc2;
  matched = fscanf(file, "%hhu", &uc);
  if (!(matched == 1 && uc == 0))
    return -51;
  SKIP_LINE(file);

  matched = fscanf(file, "%hhu", &uc);
  if (!(matched == 1 && uc == 255))
    return -52;
  SKIP_LINE(file);

  // Truncation: 256 wraps to 0 as unsigned char
  matched = fscanf(file, "%hhu", &uc);
  if (!(matched == 1 && uc == 0))
    return -53;
  SKIP_LINE(file);

  // Truncation: 257 wraps to 1 as unsigned char
  matched = fscanf(file, "%hhu", &uc);
  if (!(matched == 1 && uc == 1))
    return -54;
  SKIP_LINE(file);

  matched = fscanf(file, "%hhu %hhu", &uc, &uc2);
  if (!(matched == 2 && uc == 10 && uc2 == 20))
    return -55;
  SKIP_LINE(file);

  // width limits the conversion
  matched = fscanf(file, "%2hhu", &uc);
  if (!(matched == 1 && uc == 12))
    return -56;
  SKIP_LINE(file);

  // Overflow above UINT_MAX: per C semantics, the input is parsed as
  // a wide unsigned and only the low bits are stored, so 0x100000000
  // gives 0 in both u16 and u8.
  matched = fscanf(file, "%hu", &us);
  if (!(matched == 1 && us == 0))
    return -57;
  SKIP_LINE(file);

  matched = fscanf(file, "%hhu", &uc);
  if (!(matched == 1 && uc == 0))
    return -58;
  SKIP_LINE(file);

  // %hx (unsigned short, hex) truncation
  matched = fscanf(file, "%hx", &us);
  if (!(matched == 1 && us == 0xFFFF))
    return -59;
  SKIP_LINE(file);

  // Truncation: 0x10000 wraps to 0 as unsigned short
  matched = fscanf(file, "%hx", &us);
  if (!(matched == 1 && us == 0))
    return -60;
  SKIP_LINE(file);

  // Truncation: 0x10001 wraps to 1 as unsigned short
  matched = fscanf(file, "%hx", &us);
  if (!(matched == 1 && us == 1))
    return -61;
  SKIP_LINE(file);

  // %hhx (unsigned char, hex) truncation
  matched = fscanf(file, "%hhx", &uc);
  if (!(matched == 1 && uc == 0xFF))
    return -62;
  SKIP_LINE(file);

  // Truncation: 0x100 wraps to 0 as unsigned char
  matched = fscanf(file, "%hhx", &uc);
  if (!(matched == 1 && uc == 0))
    return -63;
  SKIP_LINE(file);

  // Truncation: 0x101 wraps to 1 as unsigned char
  matched = fscanf(file, "%hhx", &uc);
  if (!(matched == 1 && uc == 1))
    return -64;
  SKIP_LINE(file);

  // %hd (signed short) edge cases
  short ss, ss2;
  matched = fscanf(file, "%hd", &ss);
  if (!(matched == 1 && ss == 0))
    return -65;
  SKIP_LINE(file);

  matched = fscanf(file, "%hd", &ss);
  if (!(matched == 1 && ss == 32767))
    return -66;
  SKIP_LINE(file);

  matched = fscanf(file, "%hd", &ss);
  if (!(matched == 1 && ss == -32768))
    return -67;
  SKIP_LINE(file);

  // Truncation: 32768 wraps to -32768 as signed short
  matched = fscanf(file, "%hd", &ss);
  if (!(matched == 1 && ss == -32768))
    return -68;
  SKIP_LINE(file);

  // Truncation: -32769 wraps to 32767 as signed short
  matched = fscanf(file, "%hd", &ss);
  if (!(matched == 1 && ss == 32767))
    return -69;
  SKIP_LINE(file);

  matched = fscanf(file, "%hd %hd", &ss, &ss2);
  if (!(matched == 2 && ss == -100 && ss2 == 200))
    return -70;
  SKIP_LINE(file);

  // width limits the conversion
  matched = fscanf(file, "%3hd", &ss);
  if (!(matched == 1 && ss == 123))
    return -71;
  SKIP_LINE(file);

  // width counts the sign character
  matched = fscanf(file, "%4hd", &ss);
  if (!(matched == 1 && ss == -123))
    return -72;
  SKIP_LINE(file);

  // %hhd (signed char) edge cases
  signed char sc, sc2;
  matched = fscanf(file, "%hhd", &sc);
  if (!(matched == 1 && sc == 0))
    return -73;
  SKIP_LINE(file);

  matched = fscanf(file, "%hhd", &sc);
  if (!(matched == 1 && sc == 127))
    return -74;
  SKIP_LINE(file);

  matched = fscanf(file, "%hhd", &sc);
  if (!(matched == 1 && sc == -128))
    return -75;
  SKIP_LINE(file);

  // Truncation: 128 wraps to -128 as signed char
  matched = fscanf(file, "%hhd", &sc);
  if (!(matched == 1 && sc == -128))
    return -76;
  SKIP_LINE(file);

  // Truncation: -129 wraps to 127 as signed char
  matched = fscanf(file, "%hhd", &sc);
  if (!(matched == 1 && sc == 127))
    return -77;
  SKIP_LINE(file);

  matched = fscanf(file, "%hhd %hhd", &sc, &sc2);
  if (!(matched == 2 && sc == -10 && sc2 == 20))
    return -78;
  SKIP_LINE(file);

  // width limits the conversion
  matched = fscanf(file, "%2hhd", &sc);
  if (!(matched == 1 && sc == 12))
    return -79;
  SKIP_LINE(file);

  // width counts the sign character
  matched = fscanf(file, "%3hhd", &sc);
  if (!(matched == 1 && sc == -12))
    return -80;
  SKIP_LINE(file);

  fclose(file);
  return 0;
}

int test_CGImage_JPEG() {
  FILE *file = fopen("test_1x1_black_pixel.jpg", "r");
  if (file == NULL) {
    return -1;
  }
  char buf[720];
  memset(buf, '\0', 720);
  size_t read = fread(buf, 1, 720, file);
  fclose(file);
  if (read != 720) {
    return -2;
  }
  CFDataRef dataRef = CFDataCreate(NULL, buf, sizeof(buf));
  if (dataRef == NULL) {
    return -3;
  }
  CGDataProviderRef dataProvider = CGDataProviderCreateWithCFData(dataRef);
  if (dataRef == NULL) {
    return -4;
  }
  CGImageRef imageRef = CGImageCreateWithJPEGDataProvider(
      dataProvider, NULL, 1 /* true */, 0 /* kCGRenderingIntentDefault */);
  if (imageRef == NULL) {
    return -5;
  }
  size_t width = CGImageGetWidth(imageRef);
  size_t height = CGImageGetHeight(imageRef);
  if (!(width == 1 && height == 1)) {
    return -6;
  }
  CFDataRef rawData = CGDataProviderCopyData(CGImageGetDataProvider(imageRef));
  const unsigned char *bytes = CFDataGetBytePtr(rawData);
  // Check that pixel is indeed a RGB black one
  if (!(bytes[0] == 0 && bytes[1] == 0 && bytes[2] == 0)) {
    return -7;
  }
  CFRelease(rawData);
  CFRelease(imageRef);
  CFRelease(dataProvider);
  return 0;
}

int test_CFStringFind() {
  CFStringRef a =
      CFStringCreateWithCString(NULL, "/a/b/c/b", kCFStringEncodingASCII);
  CFStringRef b = CFStringCreateWithCString(NULL, "/b", kCFStringEncodingASCII);
  CFStringRef d = CFStringCreateWithCString(NULL, "/d", kCFStringEncodingASCII);
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

// === flockfile / funlockfile tests ===

int test_flockfile_basic() {
  FILE *file = fopen("TestApp", "r");
  if (file == NULL)
    return -1;

  flockfile(file);
  funlockfile(file);

  fclose(file);
  return 0;
}

// flockfile is required to be recursive: the same thread may acquire the
// lock multiple times and must release it the same number of times.
int test_flockfile_recursive() {
  FILE *file = fopen("TestApp", "r");
  if (file == NULL)
    return -1;

  flockfile(file);
  flockfile(file);
  flockfile(file);
  funlockfile(file);
  funlockfile(file);
  funlockfile(file);

  // After unlocking the matching number of times, the stream must be
  // available again, so ftrylockfile must succeed.
  if (ftrylockfile(file) != 0) {
    fclose(file);
    return -2;
  }
  funlockfile(file);

  fclose(file);
  return 0;
}

int test_ftrylockfile_unlocked() {
  FILE *file = fopen("TestApp", "r");
  if (file == NULL)
    return -1;

  if (ftrylockfile(file) != 0) {
    fclose(file);
    return -2;
  }
  funlockfile(file);

  fclose(file);
  return 0;
}

struct ftrylockfile_args {
  FILE *file;
  int result;
};

void *ftrylockfile_other_thread(void *arg) {
  struct ftrylockfile_args *a = arg;
  a->result = ftrylockfile(a->file);
  // If we somehow obtained the lock (we shouldn't), release it so the
  // main thread is not left blocked.
  if (a->result == 0)
    funlockfile(a->file);
  return NULL;
}

// When a stream is locked by one thread, ftrylockfile from another thread
// must fail (return non-zero).
int test_ftrylockfile_locked_by_other_thread() {
  FILE *file = fopen("TestApp", "r");
  if (file == NULL)
    return -1;

  flockfile(file);

  struct ftrylockfile_args args = {file, -1};
  pthread_t p;
  if (pthread_create(&p, NULL, ftrylockfile_other_thread, &args) != 0) {
    funlockfile(file);
    fclose(file);
    return -2;
  }
  if (pthread_join(p, NULL) != 0) {
    funlockfile(file);
    fclose(file);
    return -3;
  }

  funlockfile(file);
  fclose(file);

  if (args.result == 0)
    return -4;
  return 0;
}

struct flockfile_blocking_args {
  FILE *file;
  pthread_mutex_t *mu;
  pthread_cond_t *cv;
  int *started;
  int *acquired;
};

void *flockfile_blocking_thread(void *arg) {
  struct flockfile_blocking_args *a = arg;

  // Announce that we are about to attempt flockfile, so the main thread
  // does not have to rely on a sleep to know we have made progress.
  pthread_mutex_lock(a->mu);
  *(a->started) = 1;
  pthread_cond_signal(a->cv);
  pthread_mutex_unlock(a->mu);

  // This call must block until the main thread releases the stream lock.
  flockfile(a->file);

  pthread_mutex_lock(a->mu);
  *(a->acquired) = 1;
  pthread_cond_signal(a->cv);
  pthread_mutex_unlock(a->mu);

  funlockfile(a->file);
  return NULL;
}

// flockfile must block another thread until the lock is released by the
// thread that currently owns it.
int test_flockfile_blocks_other_thread() {
  FILE *file = fopen("TestApp", "r");
  if (file == NULL)
    return -1;

  pthread_mutex_t mu;
  if (pthread_mutex_init(&mu, NULL) != 0) {
    fclose(file);
    return -2;
  }
  pthread_cond_t cv;
  if (pthread_cond_init(&cv, NULL) != 0) {
    pthread_mutex_destroy(&mu);
    fclose(file);
    return -3;
  }

  int started = 0;
  int acquired = 0;
  struct flockfile_blocking_args args = {file, &mu, &cv, &started, &acquired};

  flockfile(file);

  pthread_t p;
  if (pthread_create(&p, NULL, flockfile_blocking_thread, &args) != 0) {
    funlockfile(file);
    pthread_cond_destroy(&cv);
    pthread_mutex_destroy(&mu);
    fclose(file);
    return -4;
  }

  // Wait until the worker thread has reached the point right before
  // flockfile, then yield so the scheduler can run it into the blocking
  // call.
  pthread_mutex_lock(&mu);
  while (!started)
    pthread_cond_wait(&cv, &mu);
  pthread_mutex_unlock(&mu);
  sched_yield();

  pthread_mutex_lock(&mu);
  int acquired_before_unlock = acquired;
  pthread_mutex_unlock(&mu);

  funlockfile(file);

  // Once we have released the stream lock, the worker must eventually be
  // able to acquire it. Wait for that signal rather than for thread join
  // so that a hang in flockfile is attributed to this assertion.
  pthread_mutex_lock(&mu);
  while (!acquired)
    pthread_cond_wait(&cv, &mu);
  pthread_mutex_unlock(&mu);

  if (pthread_join(p, NULL) != 0) {
    pthread_cond_destroy(&cv);
    pthread_mutex_destroy(&mu);
    fclose(file);
    return -5;
  }

  pthread_cond_destroy(&cv);
  pthread_mutex_destroy(&mu);
  fclose(file);

  if (acquired_before_unlock != 0)
    return -6;
  if (acquired != 1)
    return -7;
  return 0;
}

// flockfile is most commonly used to make a sequence of stdio calls atomic
// from the perspective of other threads. Verify that the main stdio entry
// points work normally while the current thread holds the stream lock.
int test_flockfile_io_while_locked() {
  FILE *file = fopen("TestApp", "r");
  if (file == NULL)
    return -1;

  flockfile(file);

  // ftello must succeed and report the initial position.
  if (ftello(file) != 0) {
    funlockfile(file);
    fclose(file);
    return -2;
  }

  // fread must succeed and advance the position.
  char buf[8];
  size_t n = fread(buf, 1, sizeof(buf), file);
  if (n != sizeof(buf)) {
    funlockfile(file);
    fclose(file);
    return -3;
  }
  if (ftello(file) != (off_t)sizeof(buf)) {
    funlockfile(file);
    fclose(file);
    return -4;
  }

  // fseeko must succeed and reset the position.
  if (fseeko(file, 0, SEEK_SET) != 0) {
    funlockfile(file);
    fclose(file);
    return -5;
  }
  if (ftello(file) != 0) {
    funlockfile(file);
    fclose(file);
    return -6;
  }

  // feof / clearerr / fflush / fileno must not abort while the lock is
  // held by the calling thread.
  (void)feof(file);
  clearerr(file);
  (void)fflush(file);
  if (fileno(file) < 0) {
    funlockfile(file);
    fclose(file);
    return -7;
  }

  funlockfile(file);
  fclose(file);
  return 0;
}

// === end flockfile / funlockfile tests ===

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

int test_close() {
  if (close(0) != 0)
    return -1;
  if (close(-1) == 0)
    return -2;
  if (close(1000) == 0)
    return -3;
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

  CFStringRef key =
      CFStringCreateWithCString(NULL, "Key", kCFStringEncodingASCII);
  CFStringRef value =
      CFStringCreateWithCString(NULL, "Value", kCFStringEncodingASCII);
  if (key == NULL || value == NULL) {
    CFRelease(key);
    CFRelease(value);
    CFRelease(dict);
    return -2;
  }

  // Create copies to be stored in the dictionary
  CFStringRef key1 =
      CFStringCreateWithCString(NULL, "Key", kCFStringEncodingASCII);
  CFStringRef value1 =
      CFStringCreateWithCString(NULL, "Value", kCFStringEncodingASCII);

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

  CFStringRef valueNew =
      CFStringCreateWithCString(NULL, "NewValue", kCFStringEncodingASCII);
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

int test_fesetround() {
  int default_rounding = fegetround();
  if (default_rounding != FE_TONEAREST) {
    return -1;
  }
  if (lrint(+11.5) != +12.0 || lrint(+12.5) != +12.0 || lrint(-11.5) != -12.0) {
    return -2;
  }
  int res = fesetround(FE_TOWARDZERO);
  if (res != 0) {
    return -3;
  }
  if (lrint(+11.5) != +11.0 || lrint(+12.5) != +12.0 || lrint(-11.5) != -11.0) {
    return -4;
  }
  res = fesetround(default_rounding);
  if (res != 0) {
    return -5;
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

int test_frexp() {
  double value, frac;
  int exp;

  // Test 1: 0.0 -> should return 0.0 and exponent 0.
  value = 0.0;
  frac = frexp(value, &exp);
  if (frac != 0.0 || exp != 0) {
    return -1;
  }

  // Test 2: 8.0 -> 8.0 = 0.5 * 2^4, so fraction 0.5 and exponent 4.
  value = 8.0;
  frac = frexp(value, &exp);
  if (frac != 0.5 || exp != 4) {
    return -2;
  }

  // Test 3: 0.75 -> already normalized, should return 0.75 and exponent 0.
  value = 0.75;
  frac = frexp(value, &exp);
  if (frac != 0.75 || exp != 0) {
    return -3;
  }

  // Test 4: -4.0 -> -4.0 = -0.5 * 2^3, so fraction -0.5 and exponent 3.
  value = -4.0;
  frac = frexp(value, &exp);
  if (frac != -0.5 || exp != 3) {
    return -4;
  }

  // Test 5: 1.0 -> 1.0 = 0.5 * 2^1, so fraction 0.5 and exponent 1.
  value = 1.0;
  frac = frexp(value, &exp);
  if (frac != 0.5 || exp != 1) {
    return -5;
  }

  // Test 6: pi -> 3.141592653589793 = (pi/4) * 2^2, expect fraction
  // ~0.7853981633974483 and exponent 2.
  value = 3.141592653589793;
  frac = frexp(value, &exp);
  if (exp != 2 || fabs(frac - (3.141592653589793 / 4.0)) > 1e-15) {
    return -6;
  }

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

int test_inet_addr() {
  unsigned int res = inet_addr("127.0.0.1");
  if (res != 16777343) {
    return -1;
  }
  return 0;
}

int test_inet_ntop() {
  struct in_addr addr;
  char buffer[16]; // INET_ADDRSTRLEN

  unsigned int res = inet_addr("127.0.0.1");
  if (res != 16777343) {
    return -1;
  }

  addr.s_addr = res;
  if (inet_ntop(2, &addr, buffer, sizeof(buffer)) == NULL) {
    return -2;
  }

  if (strcmp(buffer, "127.0.0.1") != 0) {
    return -3;
  }

  return 0;
}

int test_inet_pton() {
  const char *ip_str = "127.0.0.1";
  struct in_addr addr;

  int res = inet_pton(2, ip_str, &addr);
  if (res <= 0) {
    return -1;
  }
  if (addr.s_addr != 16777343) {
    return -2;
  }
  return 0;
}

int test_case_CFURL(const char *basePathCStr, const char *urlPathCStr,
                    const char *fileNameCStr,
                    const char *expectedAppendedCStr) {
  CFURLRef url = CFURLCreateFromFileSystemRepresentation(
      NULL, (uint8_t *)urlPathCStr, strlen(urlPathCStr),
      1 // isDirectory
  );
  if (url == NULL) {
    return -1;
  }

  CFStringRef fileName =
      CFStringCreateWithCString(NULL, fileNameCStr, kCFStringEncodingASCII);
  CFURLRef appendedURL =
      CFURLCreateCopyAppendingPathComponent(NULL, url, fileName,
                                            0 // isDirectory
      );
  CFRelease(fileName);
  if (appendedURL == NULL) {
    CFRelease(url);
    return -2;
  }

  CFStringRef gotPath =
      CFURLCopyFileSystemPath(appendedURL, 0); // kCFURLPOSIXPathStyle
  if (gotPath == NULL) {
    CFRelease(appendedURL);
    CFRelease(url);
    return -3;
  }

  CFStringRef expectedAppended = CFStringCreateWithCString(
      NULL, expectedAppendedCStr, kCFStringEncodingASCII);
  if (!CFEqual(gotPath, expectedAppended)) {
    CFRelease(expectedAppended);
    CFRelease(gotPath);
    CFRelease(appendedURL);
    CFRelease(url);
    return -4;
  }
  CFRelease(expectedAppended);
  CFRelease(gotPath);

  CFURLRef deletedURL =
      CFURLCreateCopyDeletingLastPathComponent(NULL, appendedURL);
  if (deletedURL == NULL) {
    CFRelease(appendedURL);
    CFRelease(url);
    return -5;
  }

  gotPath = CFURLCopyFileSystemPath(deletedURL, 0); // kCFURLPOSIXPathStyle
  if (gotPath == NULL) {
    CFRelease(deletedURL);
    CFRelease(appendedURL);
    CFRelease(url);
    return -6;
  }

  CFStringRef expectedBase =
      CFStringCreateWithCString(NULL, basePathCStr, kCFStringEncodingASCII);
  if (!CFEqual(gotPath, expectedBase)) {
    CFRelease(expectedBase);
    CFRelease(gotPath);
    CFRelease(deletedURL);
    CFRelease(appendedURL);
    CFRelease(url);
    return -7;
  }

  CFRelease(expectedBase);
  CFRelease(gotPath);
  CFRelease(deletedURL);
  CFRelease(appendedURL);
  CFRelease(url);

  return 0;
}

int test_CFURL() {
  // base path, url path, filename, expected path
  int res = test_case_CFURL("/a/b/c", "/a/b/c", "test.txt", "/a/b/c/test.txt");
  if (res != 0) {
    return res;
  }
  res = test_case_CFURL("/a/b/c", "/a/b/c/", "test.txt", "/a/b/c/test.txt");
  if (res != 0) {
    return res - 10;
  }
  res = test_case_CFURL("/a/b/c", "/a/b/c/", "test.txt", "/a/b/c/test.txt");
  if (res != 0) {
    return res - 20;
  }
  return 0;
}

int test_CFNumberCompare_simple() {
  float a = 3.333;
  CFNumberRef aa = CFNumberCreate(NULL, 5, &a); // kCFNumberFloat32Type
  double b = 3.333;
  CFNumberRef bb = CFNumberCreate(NULL, 6, &b); // kCFNumberFloat64Type
  CFComparisonResult res = CFNumberCompare(aa, bb, NULL);
  // `3.333` looses precision as float, thus 2 numbers are not equal
  if (res != kCFCompareLessThan) {
    return -1;
  }
  res = CFNumberCompare(bb, aa, NULL);
  if (res != kCFCompareGreaterThan) {
    return -2;
  }
  int c = -1;
  CFNumberRef cc = CFNumberCreate(NULL, 3, &c); // kCFNumberSInt32Type
  long long d = -1;
  CFNumberRef dd = CFNumberCreate(NULL, 4, &d); // kCFNumberSInt64Type
  res = CFNumberCompare(cc, dd, NULL);
  if (res != kCFCompareEqualTo) {
    return -3;
  }
  char e = 0;
  CFNumberRef ee = CFNumberCreate(NULL, 1, &e); // kCFNumberSInt8Type
  double f = 0.0;
  CFNumberRef ff = CFNumberCreate(NULL, 6, &f); // kCFNumberFloat64Type
  res = CFNumberCompare(ee, ff, NULL);
  if (res != kCFCompareEqualTo) {
    return -4;
  }
  return 0;
}

#ifndef kCFNumberSInt8Type
#define kCFNumberSInt8Type 1
#define kCFNumberSInt16Type 2
#define kCFNumberSInt32Type 3
#define kCFNumberSInt64Type 4
#define kCFNumberFloat32Type 5
#define kCFNumberFloat64Type 6
#endif

static int cmp(CFNumberRef a, CFNumberRef b, CFComparisonResult expected,
               const char *label, int failCode) {
  CFComparisonResult r = CFNumberCompare(a, b, NULL);
  if (r != expected) {
    const char *expStr = expected == kCFCompareLessThan      ? "<"
                         : expected == kCFCompareGreaterThan ? ">"
                                                             : "==";
    const char *gotStr = r == kCFCompareLessThan      ? "<"
                         : r == kCFCompareGreaterThan ? ">"
                                                      : "==";
    printf("FAIL (%d): %s : expected %s, got %s\n", failCode, label, expStr,
           gotStr);
    return failCode;
  }
  return 0;
}

#define MAKE_NUM(var, typeEnum) CFNumberCreate(NULL, typeEnum, &(var))
#define TEST_CMP(aRef, bRef, expected, label, code)                            \
  {                                                                            \
    int _e = cmp(aRef, bRef, expected, label, code);                           \
    if (_e) {                                                                  \
      CFRelease(aRef);                                                         \
      CFRelease(bRef);                                                         \
      return _e;                                                               \
    }                                                                          \
    CFRelease(aRef);                                                           \
    CFRelease(bRef);                                                           \
  }

static int compare_integral_examples(void) {
  /* Cross-width equalities */
  {
    int32_t v32 = -1;
    int64_t v64 = -1;
    CFNumberRef n32 = MAKE_NUM(v32, kCFNumberSInt32Type);
    CFNumberRef n64 = MAKE_NUM(v64, kCFNumberSInt64Type);
    TEST_CMP(n32, n64, kCFCompareEqualTo, "SInt32 -1 == SInt64 -1", -10);
  }
  {
    int8_t z8 = 0;
    double zD = 0.0;
    CFNumberRef n8 = MAKE_NUM(z8, kCFNumberSInt8Type);
    CFNumberRef nD = MAKE_NUM(zD, kCFNumberFloat64Type);
    TEST_CMP(n8, nD, kCFCompareEqualTo, "SInt8 0 == Float64 0.0", -11);
  }

  /* Min / Max ordering across widths */
  {
    int64_t min64 = INT64_MIN;
    int32_t min32 = INT32_MIN;
    CFNumberRef n64 = MAKE_NUM(min64, kCFNumberSInt64Type);
    CFNumberRef n32 = MAKE_NUM(min32, kCFNumberSInt32Type);
    TEST_CMP(n64, n32, kCFCompareLessThan, "INT64_MIN < INT32_MIN", -12);
  }
  {
    int64_t max64 = INT64_MAX;
    int32_t max32 = INT32_MAX;
    CFNumberRef n64 = MAKE_NUM(max64, kCFNumberSInt64Type);
    CFNumberRef n32 = MAKE_NUM(max32, kCFNumberSInt32Type);
    TEST_CMP(n64, n32, kCFCompareGreaterThan, "INT64_MAX > INT32_MAX", -13);
  }
  {
    int16_t min16 = INT16_MIN; /* -32768 */
    int8_t min8 = INT8_MIN;    /* -128   */
    CFNumberRef n16 = MAKE_NUM(min16, kCFNumberSInt16Type);
    CFNumberRef n8 = MAKE_NUM(min8, kCFNumberSInt8Type);
    TEST_CMP(n16, n8, kCFCompareLessThan, "INT16_MIN < INT8_MIN", -14);
  }
  {
    int16_t max16 = INT16_MAX;
    int8_t max8 = INT8_MAX;
    CFNumberRef n16 = MAKE_NUM(max16, kCFNumberSInt16Type);
    CFNumberRef n8 = MAKE_NUM(max8, kCFNumberSInt8Type);
    TEST_CMP(n16, n8, kCFCompareGreaterThan, "INT16_MAX > INT8_MAX", -15);
  }

  /* Extremes vs -1 */
  {
    int64_t min64 = INT64_MIN;
    int64_t neg1 = -1;
    CFNumberRef nMin = MAKE_NUM(min64, kCFNumberSInt64Type);
    CFNumberRef nNeg1 = MAKE_NUM(neg1, kCFNumberSInt64Type);
    TEST_CMP(nMin, nNeg1, kCFCompareLessThan, "INT64_MIN < -1", -16);
  }

  return 0;
}

static int compare_precision_examples(void) {
  /* Original float vs double 3.333 */
  {
    float f = 3.333f;
    double d = 3.333;
    CFNumberRef nf = MAKE_NUM(f, kCFNumberFloat32Type);
    CFNumberRef nd = MAKE_NUM(d, kCFNumberFloat64Type);
    /* float loses precision => float < double (expected) */
    TEST_CMP(nf, nd, kCFCompareLessThan, "float 3.333f < double 3.333", -20);
    /* Reverse */
    float f2 = 3.333f;
    double d2 = 3.333;
    CFNumberRef nf2 = MAKE_NUM(f2, kCFNumberFloat32Type);
    CFNumberRef nd2 = MAKE_NUM(d2, kCFNumberFloat64Type);
    TEST_CMP(nd2, nf2, kCFCompareGreaterThan, "double 3.333 > float 3.333f",
             -21);
  }

  /* 0.1f vs 0.1 (0.1f rounds *up* relative to double literal 0.1) */
  {
    float f = 0.1f;
    double d = 0.1; /* double literal */
    CFNumberRef nf = MAKE_NUM(f, kCFNumberFloat32Type);
    CFNumberRef nd = MAKE_NUM(d, kCFNumberFloat64Type);
    /* 0.1f (promoted) is slightly greater than 0.1 double */
    TEST_CMP(nf, nd, kCFCompareGreaterThan, "0.1f > 0.1 (double)", -22);
  }

  /* INT64_MAX vs its double representation (double rounds) */
  {
    int64_t i = INT64_MAX;        /*  9223372036854775807 */
    double d = (double)INT64_MAX; /* Rounds to 9223372036854775808 */
    CFNumberRef ni = MAKE_NUM(i, kCFNumberSInt64Type);
    CFNumberRef nd = MAKE_NUM(d, kCFNumberFloat64Type);
    TEST_CMP(ni, nd, kCFCompareLessThan,
             "INT64_MAX (exact) < double(INT64_MAX) (rounded up)", -23);
  }

  return 0;
}

static int compare_special_float_values(void) {
  /* Positive vs negative zero */
  {
    double pz = 0.0;
    double nz = -0.0;
    CFNumberRef nP = MAKE_NUM(pz, kCFNumberFloat64Type);
    CFNumberRef nN = MAKE_NUM(nz, kCFNumberFloat64Type);
    TEST_CMP(nP, nN, kCFCompareEqualTo, "+0.0 == -0.0", -24);
  }

  /* Infinities */
  {
    double inf = INFINITY;
    double ninf = -INFINITY;
    double zero = 0.0;

    CFNumberRef nInf = MAKE_NUM(inf, kCFNumberFloat64Type);
    CFNumberRef nZero = MAKE_NUM(zero, kCFNumberFloat64Type);
    TEST_CMP(nInf, nZero, kCFCompareGreaterThan, "Inf  > 0", -25);

    nZero = MAKE_NUM(zero, kCFNumberFloat64Type);
    CFNumberRef nNInf = MAKE_NUM(ninf, kCFNumberFloat64Type);
    TEST_CMP(nNInf, nZero, kCFCompareLessThan, "-Inf < 0", -26);

    nInf = MAKE_NUM(inf, kCFNumberFloat64Type);
    nNInf = MAKE_NUM(ninf, kCFNumberFloat64Type);
    TEST_CMP(nInf, nNInf, kCFCompareGreaterThan, "Inf  > -Inf", -27);
  }

  return 0;
}

static int compare_unsigned_limit_examples(void) {
  /* UINT64_MAX cannot be stored exactly as a signed 64-bit CFNumber.
     We *demonstrate* by comparing a double approximation vs INT64_MAX. */
  {
    double u64d = (double)UINT64_MAX; /* ~1.844674407e19 (loses low bits) */
    int64_t i64max = INT64_MAX;       /*  9.223372036854775807e18 */
    CFNumberRef nUApprox = MAKE_NUM(u64d, kCFNumberFloat64Type);
    CFNumberRef nI64Max = MAKE_NUM(i64max, kCFNumberSInt64Type);
    TEST_CMP(nUApprox, nI64Max, kCFCompareGreaterThan,
             "double(UINT64_MAX) > INT64_MAX", -28);
  }

  /* Similar for smaller widths: compare UINT32_MAX via double vs INT32_MAX
   * (exact vs rounding) */
  {
    double u32d = (double)UINT32_MAX; /* 4294967295 exactly representable */
    int32_t s32max = INT32_MAX;       /* 2147483647 */
    CFNumberRef nU = MAKE_NUM(u32d, kCFNumberFloat64Type);
    CFNumberRef nS = MAKE_NUM(s32max, kCFNumberSInt32Type);
    TEST_CMP(nU, nS, kCFCompareGreaterThan, "double(UINT32_MAX) > INT32_MAX",
             -29);
  }

  /* UINT8_MAX vs INT8_MAX using a wider signed container (int16) for 255 */
  {
    int16_t u8max_as16 = 255; /* representable */
    int8_t s8max = INT8_MAX;  /* 127 */
    CFNumberRef nU = MAKE_NUM(u8max_as16, kCFNumberSInt16Type);
    CFNumberRef nS = MAKE_NUM(s8max, kCFNumberSInt8Type);
    TEST_CMP(nU, nS, kCFCompareGreaterThan, "255 (as SInt16) > INT8_MAX", -30);
  }

  return 0;
}

int test_CFNumberCompare_extended(void) {
  int r;

  r = compare_integral_examples();
  if (r)
    return r;
  r = compare_precision_examples();
  if (r)
    return r;
  r = compare_special_float_values();
  if (r)
    return r;
  r = compare_unsigned_limit_examples();
  if (r)
    return r;

  return 0;
}

int test_memset_pattern() {
  char buf[64];
  // memset_pattern4
  memset_pattern4(buf, "1234", sizeof(buf));
  if (strncmp(buf, "1234123412", 10) != 0) {
    return -1;
  }
  memset(buf, 0, sizeof(buf));
  memset_pattern4(buf, "abcd", 8);
  if (memcmp(buf, "abcdabcd", 8) != 0) {
    return -2;
  }
  memset(buf, 0, sizeof(buf));
  memset_pattern4(buf, "XYZW", 3);
  if (memcmp(buf, "XYZ", 3) != 0) {
    return -3;
  }
  char original_buf[sizeof(buf)];
  memset(buf, 0xAA, sizeof(buf)); // Fill buffer with a known value
  memcpy(original_buf, buf, sizeof(buf));
  memset_pattern4(buf, "1234", 0);
  if (memcmp(buf, original_buf, sizeof(buf)) != 0) {
    return -4;
  }
  memset(buf, 0, sizeof(buf));
  char pattern4_null[] = {'A', '\0', 'B', 'C'};
  char expected4_null[] = {'A', '\0', 'B', 'C', 'A', '\0', 'B'};
  memset_pattern4(buf, pattern4_null, 7);
  if (memcmp(buf, expected4_null, 7) != 0) {
    return -5;
  }
  // memset_pattern8
  unsigned long long pattern8 = 0x0102030405060708;
  char expected8_full[] = "\x08\x07\x06\x05\x04\x03\x02\x01";
  memset(buf, 0, sizeof(buf));
  memset_pattern8(buf, &pattern8, 10);
  if (memcmp(buf, expected8_full, 8) != 0 ||
      memcmp(buf + 8, expected8_full, 2) != 0) {
    return -6;
  }
  memset(buf, 0, sizeof(buf));
  memset_pattern8(buf, &pattern8, 16);
  if (memcmp(buf, expected8_full, 8) != 0 ||
      memcmp(buf + 8, expected8_full, 8) != 0) {
    return -7;
  }
  memset(buf, 0, sizeof(buf));
  memset_pattern8(buf, &pattern8, 5);
  if (memcmp(buf, expected8_full, 5) != 0) {
    return -8;
  }
  // memset_pattern16
  const char *pattern16 = "0123456789ABCDEF";
  memset(buf, 0, sizeof(buf));
  memset_pattern16(buf, pattern16, 20);
  char expected16_trunc[] = "0123456789ABCDEF0123";
  if (memcmp(buf, expected16_trunc, 20) != 0) {
    return -9;
  }
  memset(buf, 0, sizeof(buf));
  memset_pattern16(buf, pattern16, 32);
  char expected16_exact[] = "0123456789ABCDEF0123456789ABCDEF";
  if (memcmp(buf, expected16_exact, 32) != 0) {
    return -10;
  }
  return 0;
}
typedef struct {
  SyncTester *tester;
  BOOL res;
} sync_test_arg;

void *modify(sync_test_arg *arg) {
  SyncTester *tester = arg->tester;
  arg->res = [tester holdAndCheckCounter];
  return NULL;
}
void *try_modify(SyncTester *tester) {
  [tester tryModifyCounter];
  return NULL;
}

int test_synchronized() {
  SyncTester *sync_test = [SyncTester new];
  sync_test_arg *arg = malloc(sizeof(sync_test_arg));
  memset(arg, 0, sizeof(sync_test_arg));
  arg->tester = sync_test;
  pthread_t locking_thread;
  pthread_create(&locking_thread, NULL, (void *(*)(void *)) & modify, arg);
  pthread_t blocked_threads[10];
  for (int i = 0; i < 10; i++) {
    pthread_create(blocked_threads + i, NULL, (void *(*)(void *)) & try_modify,
                   sync_test);
  }
  if (pthread_join(locking_thread, NULL))
    return -1;
  if (!arg->res)
    return -1;
  [sync_test recursiveSyncEnter];
  if (!sync_test.test_ok)
    return -1;
  return 0;
}

bool test_case_CFURLHasDirectoryPath(const char *str) {
  CFURLRef url = CFURLCreateWithBytes(NULL, str, strlen(str),
                                      kCFStringEncodingASCII, NULL);

  if (!url) {
    return false;
  }

  Boolean res = CFURLHasDirectoryPath(url);
  CFRelease(url);
  return res;
}

int test_CFURLHasDirectoryPath() {
  if (test_case_CFURLHasDirectoryPath("/a/b"))
    return -1;
  if (!test_case_CFURLHasDirectoryPath("/a/b/"))
    return -2;
  if (!test_case_CFURLHasDirectoryPath("/"))
    return -3;
  if (test_case_CFURLHasDirectoryPath("//"))
    return -4;
  if (test_case_CFURLHasDirectoryPath("//a"))
    return -5;
  if (!test_case_CFURLHasDirectoryPath("//a/"))
    return -6;
  if (!test_case_CFURLHasDirectoryPath("///"))
    return -7;
  if (!test_case_CFURLHasDirectoryPath("////"))
    return -8;
  if (!test_case_CFURLHasDirectoryPath("."))
    return -9;
  if (!test_case_CFURLHasDirectoryPath(".."))
    return -10;
  if (test_case_CFURLHasDirectoryPath("..."))
    return -11;
  if (!test_case_CFURLHasDirectoryPath("/.."))
    return -12;
  if (test_case_CFURLHasDirectoryPath(""))
    return -13;
  return 0;
}

int test_NSMutableString_deleteCharactersInRange() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];
  NSMutableString *str = [NSMutableString stringWithUTF8String:"abc"];
  NSRange r1 = {0, 3};
  [str deleteCharactersInRange:r1];
  NSString *expected = [NSString stringWithUTF8String:""];
  if (!CFEqual(str, expected)) {
    return -1;
  }
  str = [NSMutableString stringWithUTF8String:"abc"];
  NSRange r2 = {1, 1};
  [str deleteCharactersInRange:r2];
  expected = [NSString stringWithUTF8String:"ac"];
  if (!CFEqual(str, expected)) {
    return -2;
  }
  str = [NSMutableString stringWithUTF8String:"abc"];
  NSRange r3 = {0, 2};
  [str deleteCharactersInRange:r3];
  expected = [NSString stringWithUTF8String:"c"];
  if (!CFEqual(str, expected)) {
    return -3;
  }
  [pool drain];
  return 0;
}

int test_NSString_stringByReplacingOccurrencesOfString() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  // Simple replacement
  NSString *str = [NSString stringWithUTF8String:"hello world"];
  NSString *target = [NSString stringWithUTF8String:"world"];
  NSString *replacement = [NSString stringWithUTF8String:"touchHLE"];
  NSString *res = [str stringByReplacingOccurrencesOfString:target
                                                 withString:replacement];
  NSString *expected = [NSString stringWithUTF8String:"hello touchHLE"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -1;
  }

  // Multiple occurrences
  str = [NSString stringWithUTF8String:"aaa"];
  target = [NSString stringWithUTF8String:"a"];
  replacement = [NSString stringWithUTF8String:"b"];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:"bbb"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -2;
  }

  // Overlapping occurrences (should not be replaced multiple times)
  str = [NSString stringWithUTF8String:"aaaa"];
  target = [NSString stringWithUTF8String:"aa"];
  replacement = [NSString stringWithUTF8String:"b"];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:"bb"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -3;
  }

  // No occurrences
  str = [NSString stringWithUTF8String:"hello"];
  target = [NSString stringWithUTF8String:"world"];
  replacement = [NSString stringWithUTF8String:"!"];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:"hello"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -4;
  }

  // Replace with empty string
  str = [NSString stringWithUTF8String:"hello world"];
  target = [NSString stringWithUTF8String:"world"];
  replacement = [NSString stringWithUTF8String:""];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:"hello "];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -5;
  }

  // Replace whole string
  str = [NSString stringWithUTF8String:"hello"];
  target = [NSString stringWithUTF8String:"hello"];
  replacement = [NSString stringWithUTF8String:"world"];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:"world"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -6;
  }

  // Empty target (macOS behavior: returns original string)
  str = [NSString stringWithUTF8String:"hello"];
  target = [NSString stringWithUTF8String:""];
  replacement = [NSString stringWithUTF8String:"!"];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:"hello"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -7;
  }

  // Source, target and replacement empty
  str = [NSString stringWithUTF8String:""];
  target = [NSString stringWithUTF8String:""];
  replacement = [NSString stringWithUTF8String:""];
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement];
  expected = [NSString stringWithUTF8String:""];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -8;
  }

  [pool drain];
  return 0;
}

int test_NSString_stringByReplacingOccurrencesOfString_options_range() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  // Case insensitive replacement
  NSString *str = [NSString stringWithUTF8String:"Hello HELLO hello"];
  NSString *target = [NSString stringWithUTF8String:"hello"];
  NSString *replacement = [NSString stringWithUTF8String:"hi"];
  NSRange range = NSMakeRange(0, 17);
  NSString *res =
      [str stringByReplacingOccurrencesOfString:target
                                     withString:replacement
                                        options:NSCaseInsensitiveSearch
                                          range:range];
  NSString *expected = [NSString stringWithUTF8String:"hi hi hi"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -1;
  }

  // Replacement within a range
  str = [NSString stringWithUTF8String:"[hello] hello [hello]"];
  target = [NSString stringWithUTF8String:"hello"];
  replacement = [NSString stringWithUTF8String:"hi"];
  range = NSMakeRange(7, 7); // Only the middle "hello"
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement
                                          options:0
                                            range:range];
  expected = [NSString stringWithUTF8String:"[hello] hi [hello]"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -2;
  }

  // Case insensitive within a range
  str = [NSString stringWithUTF8String:"AAA aaa AAA"];
  target = [NSString stringWithUTF8String:"AAA"];
  replacement = [NSString stringWithUTF8String:"B"];
  range = NSMakeRange(0, 7); // "AAA aaa"
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement
                                          options:NSCaseInsensitiveSearch
                                            range:range];
  expected = [NSString stringWithUTF8String:"B B AAA"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -3;
  }

  // Range at the end
  str = [NSString stringWithUTF8String:"hello hello"];
  target = [NSString stringWithUTF8String:"hello"];
  replacement = [NSString stringWithUTF8String:"world"];
  range = NSMakeRange(6, 5);
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement
                                          options:0
                                            range:range];
  expected = [NSString stringWithUTF8String:"hello world"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -4;
  }

  // Empty range
  str = [NSString stringWithUTF8String:"hello"];
  target = [NSString stringWithUTF8String:"hello"];
  replacement = [NSString stringWithUTF8String:"world"];
  range = NSMakeRange(2, 0);
  res = [str stringByReplacingOccurrencesOfString:target
                                       withString:replacement
                                          options:0
                                            range:range];
  expected = [NSString stringWithUTF8String:"hello"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -5;
  }

  [pool drain];
  return 0;
}

int test_NSString_pathWithComponents() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  // Absolute path
  NSArray *components =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"/"],
                                [NSString stringWithUTF8String:"a"],
                                [NSString stringWithUTF8String:"b"], nil];
  NSString *res = [NSString pathWithComponents:components];
  NSString *expected = [NSString stringWithUTF8String:"/a/b"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -1;
  }

  // Relative path
  components =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"a"],
                                [NSString stringWithUTF8String:"b"], nil];
  res = [NSString pathWithComponents:components];
  expected = [NSString stringWithUTF8String:"a/b"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -2;
  }

  // No redundant slashes
  components =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"a/"],
                                [NSString stringWithUTF8String:"/b"], nil];
  res = [NSString pathWithComponents:components];
  expected = [NSString stringWithUTF8String:"a/b"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -3;
  }

  // Single component
  components =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"a"], nil];
  res = [NSString pathWithComponents:components];
  expected = [NSString stringWithUTF8String:"a"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -4;
  }

  // Empty array
  components = [NSArray array];
  res = [NSString pathWithComponents:components];
  expected = [NSString stringWithUTF8String:""];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -5;
  }

  // Empty strings inside components
  components =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"a"],
                                [NSString stringWithUTF8String:""],
                                [NSString stringWithUTF8String:""],
                                [NSString stringWithUTF8String:"b"], nil];
  res = [NSString pathWithComponents:components];
  expected = [NSString stringWithUTF8String:"a/b"];
  if (![res isEqualToString:expected]) {
    [pool drain];
    return -6;
  }

  [pool drain];
  return 0;
}

int test_strptime() {
  struct tm tm;
  memset(&tm, 0, sizeof(struct tm));
  char *res = strptime("12:34:56,", "%H:%M:%S,", &tm);
  if (res == NULL || *res != '\0') {
    return -1;
  }
  if (tm.tm_hour != 12 || tm.tm_min != 34 || tm.tm_sec != 56) {
    return -2;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("01:02:03,", "%H:%M:%S,", &tm);
  if (res == NULL || *res != '\0') {
    return -3;
  }
  if (tm.tm_hour != 1 || tm.tm_min != 2 || tm.tm_sec != 3) {
    return -4;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("invalid", "%H:%M:%S,", &tm);
  if (res != NULL) {
    return -5;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("12:34:56,extra", "%H:%M:%S,", &tm);
  if (res == NULL || strcmp(res, "extra") != 0) {
    return -6;
  }
  if (tm.tm_hour != 12 || tm.tm_min != 34 || tm.tm_sec != 56) {
    return -7;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("12   :34: 56", "%H : %M : %S", &tm);
  if (res == NULL || *res != '\0') {
    return -8;
  }
  if (tm.tm_hour != 12 || tm.tm_min != 34 || tm.tm_sec != 56) {
    return -9;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("12:34:56", "%H :%M :%S", &tm);
  if (res == NULL || *res != '\0') {
    return -10;
  }
  if (tm.tm_hour != 12 || tm.tm_min != 34 || tm.tm_sec != 56) {
    return -11;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("12\t\n :34\f:56", "%H :%M :%S", &tm);
  if (res == NULL || *res != '\0') {
    return -12;
  }
  if (tm.tm_hour != 12 || tm.tm_min != 34 || tm.tm_sec != 56) {
    return -13;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("  12:34:56  ", " %H:%M:%S ", &tm);
  if (res == NULL || *res != '\0') {
    return -14;
  }
  if (tm.tm_hour != 12 || tm.tm_min != 34 || tm.tm_sec != 56) {
    return -15;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("XX:34:56", "%H:%M:%S", &tm);
  if (res != NULL) {
    return -16;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("12:XX:56", "%H:%M:%S", &tm);
  if (res != NULL) {
    return -17;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("12:34:XX", "%H:%M:%S", &tm);
  if (res != NULL) {
    return -18;
  }

  memset(&tm, 0, sizeof(struct tm));
  res = strptime("10\r\n", "%H:%M:%S,", &tm);
  if (res != NULL) {
    return -19;
  }

  return 0;
}

int test_strftime() {
  struct tm tm;
  char buf[64];
  memset(&tm, 0, sizeof(struct tm));
  tm.tm_mon = 0; // January
  tm.tm_mday = 31;
  tm.tm_hour = 12;
  tm.tm_min = 34;

  size_t res = strftime(buf, sizeof(buf), "%m/%d     %H:%M", &tm);
  if (res == 0) {
    return -1;
  }
  if (strcmp(buf, "01/31     12:34") != 0) {
    return -2;
  }

  memset(&tm, 0, sizeof(struct tm));
  tm.tm_mon = 10; // November
  tm.tm_mday = 5;
  tm.tm_hour = 9;
  tm.tm_min = 7;

  res = strftime(buf, sizeof(buf), "%m/%d     %H:%M", &tm);
  if (res == 0) {
    return -3;
  }
  if (strcmp(buf, "11/05     09:07") != 0) {
    return -4;
  }

  return 0;
}

@interface InvocationTarget : NSObject {
@public
  id receivedValue;
  const char *cstringValue;
  int intValue;
}
- (void)storeValue:(id)value;
- (void)clearValue;
- (void)storeCString:(const char *)str;
- (void)storeIntPtr:(int *)ptr;
@end

@implementation InvocationTarget
- (void)storeValue:(id)value {
  receivedValue = value;
}
- (void)clearValue {
  receivedValue = nil;
}
- (void)storeCString:(const char *)str {
  cstringValue = str;
}
- (void)storeIntPtr:(int *)ptr {
  intValue = *ptr;
}
@end

static BOOL g_deallocTrackerDidDealloc = NO;

@interface DeallocTracker : NSObject
@end

@implementation DeallocTracker
- (void)dealloc {
  g_deallocTrackerDidDealloc = YES;
  [super dealloc];
}
@end

int test_NSMethodSignature() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  // "v12@0:4@8" = void return, 3 args: self(@), _cmd(:), one id(@)
  NSMethodSignature *sig =
      [NSMethodSignature signatureWithObjCTypes:"v12@0:4@8"];

  if ([sig numberOfArguments] != 3) {
    [pool drain];
    return -1;
  }

  // methodReturnType should be "v"
  if (strcmp([sig methodReturnType], "v") != 0) {
    [pool drain];
    return -2;
  }

  // arg 0 = "@" (self)
  if (strcmp([sig getArgumentTypeAtIndex:0], "@") != 0) {
    [pool drain];
    return -3;
  }

  // arg 1 = ":" (SEL)
  if (strcmp([sig getArgumentTypeAtIndex:1], ":") != 0) {
    [pool drain];
    return -4;
  }

  // arg 2 = "@" (id argument)
  if (strcmp([sig getArgumentTypeAtIndex:2], "@") != 0) {
    [pool drain];
    return -5;
  }

  // "v8@0:4" = void return, 2 args: self, _cmd (no extra args)
  NSMethodSignature *sig2 = [NSMethodSignature signatureWithObjCTypes:"v8@0:4"];
  if ([sig2 numberOfArguments] != 2) {
    [pool drain];
    return -6;
  }
  if (strcmp([sig2 methodReturnType], "v") != 0) {
    [pool drain];
    return -7;
  }

  // "v12@0:4^i8" = void return, 3 args: self(@), _cmd(:), pointer-to-int(^i)
  NSMethodSignature *sig3 =
      [NSMethodSignature signatureWithObjCTypes:"v12@0:4^i8"];
  if ([sig3 numberOfArguments] != 3) {
    [pool drain];
    return -8;
  }
  if (strcmp([sig3 methodReturnType], "v") != 0) {
    [pool drain];
    return -9;
  }
  if (strcmp([sig3 getArgumentTypeAtIndex:2], "^i") != 0) {
    [pool drain];
    return -10;
  }

  [pool drain];
  return 0;
}

int test_NSInvocation() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  InvocationTarget *target = [InvocationTarget new];
  NSMethodSignature *sig =
      [NSMethodSignature signatureWithObjCTypes:"v12@0:4@8"];
  NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];

  [inv setTarget:target];
  SEL sel = NSSelectorFromString([NSString stringWithUTF8String:"storeValue:"]);
  [inv setSelector:sel];

  // setArgument:atIndex: takes a pointer to the argument value
  NSObject *val = [NSObject new];
  [inv setArgument:&val atIndex:2];
  [inv invoke];

  if (target->receivedValue != val) {
    [pool drain];
    return -1;
  }

  [pool drain];
  return 0;
}

int test_NSInvocation_invokeWithTarget() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  InvocationTarget *target1 = [InvocationTarget new];
  InvocationTarget *target2 = [InvocationTarget new];
  NSMethodSignature *sig =
      [NSMethodSignature signatureWithObjCTypes:"v12@0:4@8"];
  NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];

  [inv setTarget:target1];
  SEL sel = NSSelectorFromString([NSString stringWithUTF8String:"storeValue:"]);
  [inv setSelector:sel];

  NSObject *val = [NSObject new];
  [inv setArgument:&val atIndex:2];

  // invokeWithTarget: should use target2, not target1
  [inv invokeWithTarget:target2];

  if (target2->receivedValue != val) {
    [pool drain];
    return -1;
  }

  [pool drain];
  return 0;
}

int test_NSInvocation_retainArguments() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  // Test 1: object argument is retained by the invocation.
  {
    InvocationTarget *target = [InvocationTarget new];
    NSMethodSignature *sig =
        [NSMethodSignature signatureWithObjCTypes:"v12@0:4@8"];
    NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];
    [inv setTarget:target];
    SEL sel =
        NSSelectorFromString([NSString stringWithUTF8String:"storeValue:"]);
    [inv setSelector:sel];

    NSObject *val = [[NSObject alloc] init]; // retainCount = 1
    NSUInteger before = [val retainCount];
    [inv setArgument:&val atIndex:2];
    [inv retainArguments]; // invocation must retain val
    if ([val retainCount] != before + 1) {
      [pool drain];
      return -1;
    }
    // Invoke still works; val is alive because the invocation holds it.
    [inv invoke];
    if (target->receivedValue != val) {
      [pool drain];
      return -2;
    }
    [val release]; // balance our alloc; invocation still holds one ref
  }

  // Test 2: C string argument is copied so the invocation owns the bytes.
  {
    InvocationTarget *target = [InvocationTarget new];
    // "v12@0:4*8" = void, self(@), _cmd(:), const char *(*)
    NSMethodSignature *sig =
        [NSMethodSignature signatureWithObjCTypes:"v12@0:4*8"];
    NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];
    [inv setTarget:target];
    SEL sel =
        NSSelectorFromString([NSString stringWithUTF8String:"storeCString:"]);
    [inv setSelector:sel];

    char buf[16];
    strcpy(buf, "hello");
    const char *ptr = buf;
    [inv setArgument:&ptr atIndex:2];
    [inv retainArguments]; // must copy "hello" into invocation-owned memory

    // Overwrite original buffer; invocation must pass its copy, not buf.
    strcpy(buf, "world");

    [inv invoke];
    if (strcmp(target->cstringValue, "hello") != 0) {
      [pool drain];
      return -3;
    }
  }

  // Test 3: invocation keeps @ arg alive after caller drops its reference.
  {
    InvocationTarget *target = [InvocationTarget new];
    NSMethodSignature *sig =
        [NSMethodSignature signatureWithObjCTypes:"v12@0:4@8"];
    NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];
    [inv setTarget:target];
    SEL sel =
        NSSelectorFromString([NSString stringWithUTF8String:"storeValue:"]);
    [inv setSelector:sel];

    g_deallocTrackerDidDealloc = NO;
    DeallocTracker *val = [[DeallocTracker alloc] init];
    DeallocTracker *weakVal = val; // un-retained alias
    [inv setArgument:&val atIndex:2];
    [inv retainArguments]; // invocation owns its own reference now
    [val release];         // caller drops its reference
    val = nil;

    // Without the invocation's retain, val would now be deallocated.
    if (g_deallocTrackerDidDealloc) {
      [pool drain];
      return -4;
    }
    [inv invoke];
    if (target->receivedValue != weakVal) {
      [pool drain];
      return -5;
    }
    if (g_deallocTrackerDidDealloc) {
      [pool drain];
      return -6;
    }
  }

  [pool drain];
  return 0;
}

int test_NSInvocation_pointer() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  InvocationTarget *target = [InvocationTarget new];
  // "v12@0:4^i8" = void, self(@), _cmd(:), int *(^i)
  NSMethodSignature *sig =
      [NSMethodSignature signatureWithObjCTypes:"v12@0:4^i8"];
  NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];

  [inv setTarget:target];
  SEL sel =
      NSSelectorFromString([NSString stringWithUTF8String:"storeIntPtr:"]);
  [inv setSelector:sel];

  int x = 42;
  int *ptr = &x;
  [inv setArgument:&ptr atIndex:2];
  [inv invoke];

  if (target->intValue != 42) {
    [pool drain];
    return -1;
  }

  [pool drain];
  return 0;
}

@interface CharBufferObject : NSObject {
@public
  char *buffer;
  NSUInteger length;

  char *badKeyBuffer;
  NSUInteger badKeyLength;
}
@end

@implementation CharBufferObject
- (instancetype)initWithBytes:(const char *)b length:(NSUInteger)l {
  self = [super init];
  length = l;
  buffer = b;
  badKeyLength = -1;
  badKeyBuffer = b;
  return self;
}

- (void)dealloc {
  free(buffer);
}

- (void)encodeWithCoder:(NSCoder *)coder {
  [coder encodeBytes:buffer
              length:length
              forKey:[NSString stringWithUTF8String:"buffer"]];
}

- (instancetype)initWithCoder:(NSCoder *)coder {
  self = [super init];
  char *temp_buffer =
      [coder decodeBytesForKey:[NSString stringWithUTF8String:"buffer"]
                returnedLength:&length];
  buffer = malloc(length);
  memcpy(buffer, temp_buffer, length);

  badKeyBuffer =
      [coder decodeBytesForKey:[NSString stringWithUTF8String:"badKey"]
                returnedLength:&badKeyLength];

  return self;
}
@end

@interface IntCoderObject : NSObject {
@public
  int value;
}
@end

@implementation IntCoderObject
- (instancetype)initWithValue:(int)v {
  self = [super init];
  value = v;
  return self;
}

- (void)encodeWithCoder:(NSCoder *)coder {
  [coder encodeInt:value forKey:[NSString stringWithUTF8String:"value"]];
}

- (instancetype)initWithCoder:(NSCoder *)coder {
  self = [super init];
  value = [coder decodeIntForKey:[NSString stringWithUTF8String:"value"]];
  return self;
}
@end

int test_NSKeyedArchiver_encodeIntForKey() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  int values[] = {12345, 0, -1, -12345, 0x7FFFFFFF, -0x80000000};
  int count = sizeof(values) / sizeof(int);

  for (int i = 0; i < count; i++) {
    IntCoderObject *obj = [[IntCoderObject alloc] initWithValue:values[i]];
    NSData *archivedData = [NSKeyedArchiver archivedDataWithRootObject:obj];
    IntCoderObject *unarchivedObj =
        [NSKeyedUnarchiver unarchiveObjectWithData:archivedData];

    if (unarchivedObj->value != values[i]) {
      [pool drain];
      return -(i + 1);
    }
  }

  [pool drain];
  return 0;
}

int test_NSKeyedArchiver_NSKeyedUnarchiver() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];
  char buffer[100];
  for (char i = 0; i < 100; i++) {
    buffer[i] = i;
  }
  CharBufferObject *obj = [[CharBufferObject alloc] initWithBytes:buffer
                                                           length:100];
  NSData *archivedData = [NSKeyedArchiver archivedDataWithRootObject:obj];
  CharBufferObject *unarchivedObj =
      [NSKeyedUnarchiver unarchiveObjectWithData:archivedData];
  if (unarchivedObj->length != obj->length) {
    return -1;
  }
  if (memcmp(unarchivedObj->buffer, obj->buffer, 100) != 0) {
    return -2;
  }
  if (unarchivedObj->badKeyLength != 0) {
    return -3;
  }
  if (unarchivedObj->badKeyBuffer != NULL) {
    return -4;
  }
  [pool drain];
  return 0;
}

int test_NSKeyedArchiver_NSDictionary_of_NSArray_of_NSStrings() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  NSArray *fruits =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"apple"],
                                [NSString stringWithUTF8String:"banana"],
                                [NSString stringWithUTF8String:"cherry"], nil];
  NSArray *colors =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"red"],
                                [NSString stringWithUTF8String:"green"],
                                [NSString stringWithUTF8String:"blue"], nil];
  NSArray *values = [NSArray arrayWithObjects:fruits, colors, nil];
  NSArray *keys =
      [NSArray arrayWithObjects:[NSString stringWithUTF8String:"fruits"],
                                [NSString stringWithUTF8String:"colors"], nil];
  NSDictionary *dict = [NSDictionary dictionaryWithObjects:values forKeys:keys];

  NSData *archivedData = [NSKeyedArchiver archivedDataWithRootObject:dict];
  NSDictionary *unarchivedDict =
      [NSKeyedUnarchiver unarchiveObjectWithData:archivedData];

  if (![unarchivedDict isKindOfClass:[NSDictionary class]]) {
    [pool drain];
    return -1;
  }
  if ([unarchivedDict count] != [dict count]) {
    [pool drain];
    return -2;
  }
  if (![unarchivedDict isEqualToDictionary:dict]) {
    [pool drain];
    return -3;
  }

  NSArray *unarchivedFruits =
      [unarchivedDict objectForKey:[NSString stringWithUTF8String:"fruits"]];
  if (![unarchivedFruits isKindOfClass:[NSArray class]]) {
    [pool drain];
    return -4;
  }
  if (![unarchivedFruits isEqualToArray:fruits]) {
    [pool drain];
    return -5;
  }

  NSArray *unarchivedColors =
      [unarchivedDict objectForKey:[NSString stringWithUTF8String:"colors"]];
  if (![unarchivedColors isKindOfClass:[NSArray class]]) {
    [pool drain];
    return -6;
  }
  if (![unarchivedColors isEqualToArray:colors]) {
    [pool drain];
    return -7;
  }

  for (NSUInteger i = 0; i < [unarchivedFruits count]; i++) {
    if (![[unarchivedFruits objectAtIndex:i] isKindOfClass:[NSString class]]) {
      [pool drain];
      return -8;
    }
  }

  [pool drain];
  return 0;
}

int test_NSNumber_stringValue() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  NSNumber *num;
  NSString *result;
  NSString *expected;

  // Bool: YES -> "1"
  num = [NSNumber numberWithBool:YES];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"1"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -1;
  }

  // Bool: NO -> "0"
  num = [NSNumber numberWithBool:NO];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -2;
  }

  // Int: 0 -> "0"
  num = [NSNumber numberWithInt:0];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -3;
  }

  // Int: 42 -> "42"
  num = [NSNumber numberWithInt:42];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"42"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -4;
  }

  // Int: -100 -> "-100"
  num = [NSNumber numberWithInt:-100];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-100"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -5;
  }

  // Int: INT_MAX -> "2147483647"
  num = [NSNumber numberWithInt:2147483647];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"2147483647"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -6;
  }

  // Int: INT_MIN -> "-2147483648"
  num = [NSNumber numberWithInt:-2147483648];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-2147483648"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -7;
  }

  // LongLong: 0 -> "0"
  num = [NSNumber numberWithLongLong:0LL];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -8;
  }

  // LongLong: 9999999999 -> "9999999999"
  num = [NSNumber numberWithLongLong:9999999999LL];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"9999999999"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -9;
  }

  // LongLong: -9999999999 -> "-9999999999"
  num = [NSNumber numberWithLongLong:-9999999999LL];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-9999999999"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -10;
  }

  // UnsignedInt: 0 -> "0"
  num = [NSNumber numberWithUnsignedInt:0U];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -11;
  }

  // UnsignedLongLong: 0 -> "0"
  num = [NSNumber numberWithUnsignedLongLong:0ULL];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -12;
  }

  // Float: 0.0 -> "0"
  num = [NSNumber numberWithFloat:0.0f];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -13;
  }

  // Float: 1.5 -> "1.5"
  num = [NSNumber numberWithFloat:1.5f];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"1.5"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -14;
  }

  // Float: -1.25 -> "-1.25"
  num = [NSNumber numberWithFloat:-1.25f];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-1.25"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -15;
  }

  // Double: 0.0 -> "0"
  num = [NSNumber numberWithDouble:0.0];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -16;
  }

  // Double: 1.5 -> "1.5"
  num = [NSNumber numberWithDouble:1.5];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"1.5"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -17;
  }

  // Double: -1.25 -> "-1.25"
  num = [NSNumber numberWithDouble:-1.25];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-1.25"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -18;
  }

  // Short: 0 -> "0"
  num = [NSNumber numberWithShort:0];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -19;
  }

  // Short: 100 -> "100"
  num = [NSNumber numberWithShort:100];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"100"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -20;
  }

  // Short: SHRT_MIN -> "-32768"
  num = [NSNumber numberWithShort:-32768];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-32768"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -21;
  }

  // Short: SHRT_MAX -> "32767"
  num = [NSNumber numberWithShort:32767];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"32767"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -22;
  }

  // UnsignedShort: 0 -> "0"
  num = [NSNumber numberWithUnsignedShort:0];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -23;
  }

  // Char: 0 -> "0"
  num = [NSNumber numberWithChar:0];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"0"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -24;
  }

  // Char: 65 -> "65"
  num = [NSNumber numberWithChar:65];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"65"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -25;
  }

  // Char: SCHAR_MIN -> "-128"
  num = [NSNumber numberWithChar:-128];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"-128"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -26;
  }

  // Char: SCHAR_MAX -> "127"
  num = [NSNumber numberWithChar:127];
  result = [num stringValue];
  expected = [NSString stringWithUTF8String:"127"];
  if (![result isEqualToString:expected]) {
    [pool drain];
    return -27;
  }

  [pool drain];
  return 0;
}

@interface NotificationObserver : NSObject {
@public
  int receivedCount;
  id lastNotification;
}
- (void)handleNotification:(NSNotification *)notification;
@end

@implementation NotificationObserver
- (void)handleNotification:(NSNotification *)notification {
  receivedCount++;
  [lastNotification release];
  lastNotification = [notification retain];
}
- (void)dealloc {
  [lastNotification release];
  [super dealloc];
}
@end

// When name is nil, the observer should receive notifications of any name.
int test_NSNotificationCenter_addObserver_nilName() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  NSNotificationCenter *center = [NSNotificationCenter defaultCenter];
  NotificationObserver *observer = [NotificationObserver new];
  SEL sel = NSSelectorFromString(
      [NSString stringWithUTF8String:"handleNotification:"]);

  [center addObserver:observer selector:sel name:nil object:nil];

  [center postNotificationName:[NSString stringWithUTF8String:"FirstName"]
                        object:nil];
  if (observer->receivedCount != 1) {
    [center removeObserver:observer];
    [observer release];
    [pool drain];
    return -1;
  }

  [center postNotificationName:[NSString stringWithUTF8String:"SecondName"]
                        object:nil];
  if (observer->receivedCount != 2) {
    [center removeObserver:observer];
    [observer release];
    [pool drain];
    return -2;
  }

  // The last notification's name should match the most recently posted one.
  NSString *lastName = [observer->lastNotification name];
  NSString *expectedName = [NSString stringWithUTF8String:"SecondName"];
  if (![lastName isEqualToString:expectedName]) {
    [center removeObserver:observer];
    [observer release];
    [pool drain];
    return -3;
  }

  [center removeObserver:observer];
  [observer release];
  [pool drain];
  return 0;
}

// When name is nil but object is specified, only notifications from that
// sender (with any name) should be delivered to the observer.
int test_NSNotificationCenter_addObserver_nilName_withObject() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  NSNotificationCenter *center = [NSNotificationCenter defaultCenter];
  NotificationObserver *observer = [NotificationObserver new];
  SEL sel = NSSelectorFromString(
      [NSString stringWithUTF8String:"handleNotification:"]);

  NSObject *sender = [NSObject new];
  NSObject *otherSender = [NSObject new];

  [center addObserver:observer selector:sel name:nil object:sender];

  // Notification from the matching sender should be delivered, regardless of
  // the notification's name.
  [center postNotificationName:[NSString stringWithUTF8String:"AnyName"]
                        object:sender];
  if (observer->receivedCount != 1) {
    [center removeObserver:observer];
    [sender release];
    [otherSender release];
    [observer release];
    [pool drain];
    return -1;
  }

  // Notification from a different sender should be filtered out, even though
  // name is nil.
  [center postNotificationName:[NSString stringWithUTF8String:"AnyName"]
                        object:otherSender];
  if (observer->receivedCount != 1) {
    [center removeObserver:observer];
    [sender release];
    [otherSender release];
    [observer release];
    [pool drain];
    return -2;
  }

  // A different notification name from the matching sender should still be
  // delivered.
  [center postNotificationName:[NSString stringWithUTF8String:"OtherName"]
                        object:sender];
  if (observer->receivedCount != 2) {
    [center removeObserver:observer];
    [sender release];
    [otherSender release];
    [observer release];
    [pool drain];
    return -3;
  }

  [center removeObserver:observer];
  [sender release];
  [otherSender release];
  [observer release];
  [pool drain];
  return 0;
}

// An observer registered with name=nil should be properly unregistered by
// removeObserver:, so it must not receive any further notifications.
int test_NSNotificationCenter_addObserver_nilName_removeObserver() {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];

  NSNotificationCenter *center = [NSNotificationCenter defaultCenter];
  NotificationObserver *observer = [NotificationObserver new];
  SEL sel = NSSelectorFromString(
      [NSString stringWithUTF8String:"handleNotification:"]);

  [center addObserver:observer selector:sel name:nil object:nil];

  [center postNotificationName:[NSString stringWithUTF8String:"BeforeRemove"]
                        object:nil];
  if (observer->receivedCount != 1) {
    [center removeObserver:observer];
    [observer release];
    [pool drain];
    return -1;
  }

  [center removeObserver:observer];

  [center postNotificationName:[NSString stringWithUTF8String:"AfterRemove"]
                        object:nil];
  if (observer->receivedCount != 1) {
    [observer release];
    [pool drain];
    return -2;
  }

  // Posting under a different name after removal should not deliver either.
  [center postNotificationName:[NSString stringWithUTF8String:"OtherName"]
                        object:nil];
  if (observer->receivedCount != 1) {
    [observer release];
    [pool drain];
    return -3;
  }

  [observer release];
  [pool drain];
  return 0;
}

int test_malloc_zone_basic() {
  malloc_zone_t *zone = malloc_create_zone(0, 0);
  unsigned char *p = malloc_zone_malloc(zone, 128);
  if (zone->size(zone, p) != 128) {
    return -1;
  }

  memset(p, 0xAB, 128);
  for (int i = 0; i < 128; i++) {
    if (p[i] != 0xAB) {
      malloc_zone_free(zone, p);
      malloc_destroy_zone(zone);
      return -2;
    }
  }
  malloc_zone_free(zone, p);
  malloc_destroy_zone(zone);

  return 0;
}

int test_malloc_zone_struct_dispatch() {
  malloc_zone_t *zone = malloc_default_zone();
  if (!zone)
    return -1;

  void *p = zone->malloc(zone, 128);
  if (!p)
    return -2;

  // malloc_size() uses the default zone. If the allocation did not work
  // this should cause a panic and thus fail the test.
  size_t sz = malloc_size(p);
  if (sz != 128) {
    zone->free(zone, p);
    return -3;
  }

  zone->free(zone, p);
  return 0;
}

// clang-format off
#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
#ifndef DEFINE_ME_WHEN_BUILDING_ON_MACOS
    // below tests are failing on macOS,
    // so we skip them
    FUNC_DEF(test_getcwd_chdir),
    FUNC_DEF(test_synchronized),
    FUNC_DEF(test_read_directory_as_fd),
    FUNC_DEF(test_pthread_get_stacksize_np),
    FUNC_DEF(test_cpp_virtual_inheritance),
#endif
    FUNC_DEF(test_qsort),
    FUNC_DEF(test_vsnprintf),
    FUNC_DEF(test_sscanf),
    FUNC_DEF(test_swscanf),
    FUNC_DEF(test_realloc),
    FUNC_DEF(test_valloc),
    FUNC_DEF(test_atof),
    FUNC_DEF(test_strtof),
    FUNC_DEF(test_sem),
    FUNC_DEF(test_mtsem),
    FUNC_DEF(test_thread_suspend_resume),
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
    FUNC_DEF(test_ungetc),
    FUNC_DEF(test_fscanf),
    FUNC_DEF(test_fscanf_new),
    FUNC_DEF(test_CFStringFind),
    FUNC_DEF(test_strcspn),
    FUNC_DEF(test_mbstowcs),
    FUNC_DEF(test_CFMutableString),
    FUNC_DEF(test_fwrite),
    FUNC_DEF(test_flockfile_basic),
    FUNC_DEF(test_flockfile_recursive),
    FUNC_DEF(test_ftrylockfile_unlocked),
    FUNC_DEF(test_ftrylockfile_locked_by_other_thread),
    FUNC_DEF(test_flockfile_blocks_other_thread),
    FUNC_DEF(test_flockfile_io_while_locked),
    FUNC_DEF(test_open),
    FUNC_DEF(test_close),
    FUNC_DEF(test_cond_var),
    FUNC_DEF(test_cond_var_static),
    FUNC_DEF(test_cond_timedwait_signaled_before_timeout),
    FUNC_DEF(test_cond_timedwait_past_deadline),
    FUNC_DEF(test_cond_timedwait_broadcast),
    FUNC_DEF(test_cond_timedwait_flag_not_sticky),
    FUNC_DEF(test_cond_timedwait_sibling_not_dropped),
    FUNC_DEF(test_pthread_mutex_normal),
    FUNC_DEF(test_pthread_mutex_recursive_trylock),
    FUNC_DEF(test_CFMutableDictionary_NullCallbacks),
    FUNC_DEF(test_CFMutableDictionary_CustomCallbacks_PrimitiveTypes),
    FUNC_DEF(test_CFMutableDictionary_CustomCallbacks_CFTypes),
    FUNC_DEF(test_lrint),
    FUNC_DEF(test_fesetround),
    FUNC_DEF(test_ldexp),
    FUNC_DEF(test_maskrune),
    FUNC_DEF(test_frexpf),
    FUNC_DEF(test_frexp),
    FUNC_DEF(test_setjmp),
    FUNC_DEF(test_inet_addr),
    FUNC_DEF(test_inet_ntop),
    FUNC_DEF(test_inet_pton),
    FUNC_DEF(test_CFURL),
    FUNC_DEF(test_CFNumberCompare_simple),
    FUNC_DEF(test_CFNumberCompare_extended),
    FUNC_DEF(test_memset_pattern),
    FUNC_DEF(test_CGGeometry),
    FUNC_DEF(test_CFURLHasDirectoryPath),
    FUNC_DEF(test_CGImage_JPEG),
    FUNC_DEF(test_NSMutableString_deleteCharactersInRange),
    FUNC_DEF(test_NSString_stringByReplacingOccurrencesOfString),
    FUNC_DEF(test_NSString_stringByReplacingOccurrencesOfString_options_range),
    FUNC_DEF(test_NSString_pathWithComponents),
    FUNC_DEF(test_strptime),
    FUNC_DEF(test_strftime),
    FUNC_DEF(test_RespondsToSelector),
    FUNC_DEF(test_NSKeyedArchiver_encodeIntForKey),
    FUNC_DEF(test_NSKeyedArchiver_NSKeyedUnarchiver),
    FUNC_DEF(test_NSKeyedArchiver_NSDictionary_of_NSArray_of_NSStrings),
    FUNC_DEF(test_AutoreleasePool),
    FUNC_DEF(test_NSNumber_stringValue),
    FUNC_DEF(test_NSMethodSignature),
    FUNC_DEF(test_NSInvocation),
    FUNC_DEF(test_NSInvocation_invokeWithTarget),
    FUNC_DEF(test_NSInvocation_retainArguments),
    FUNC_DEF(test_NSInvocation_pointer),
    FUNC_DEF(test_Initialize),
    FUNC_DEF(test_NSNotificationCenter_addObserver_nilName),
    FUNC_DEF(test_NSNotificationCenter_addObserver_nilName_withObject),
    FUNC_DEF(test_NSNotificationCenter_addObserver_nilName_removeObserver),
    FUNC_DEF(test_malloc_zone_basic),
    FUNC_DEF(test_malloc_zone_struct_dispatch),
};
// clang-format on

int TestApp_cli_tests_main(void) {
#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
  setbuf(stdout, NULL);
#endif

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
  return tests_run == tests_passed ? 0 : 1;
}
