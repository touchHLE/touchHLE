/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file contains the command-line automated tests. tests/integration.rs
// runs these automatically.

#include <CoreFoundation/CFBase.h>
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
#include <math.h>
#include <pthread.h>
#include <semaphore.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>

#import "SyncTester.h"

// Declare test functions from other files.

int test_AutoreleasePool(void);   // AutoReleasePoolTest.m
int test_CGAffineTransform(void); // CGAffineTransform.c

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
  // Test %d with alternative form
  str = str_format("%#.2d", 5);
  res += !!strcmp(str, "05");
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
  str = str_format("%e|%8e|%08e|%.e|%8.e|%.3e|%8.3e|%08.3e|%*e|%0*e", 0.0, 0.0,
                   0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 16, 0.0, 16, 0.0);
  res += !!strcmp(
      str,
      "0.000000e+00|0.000000e+00|0.000000e+00|0e+00|   "
      "0e+00|0.000e+00|0.000e+00|0.000e+00|    0.000000e+00|00000.000000e+00");
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
  str = str_format("%g|%8g|%08g|%.g|%8.g|%.3g|%8.3g|%08.3g|%*g|%0*g", 0.0, 0.0,
                   0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 16, 0.0, 16, 0.0);
  res += !!strcmp(
      str, "0|       0|00000000|0|       0|0|       0|00000000|               "
           "0|0000000000000000");
  free(str);
  // Test %g with trailing zeros
  str = str_format("%.14g", 1.0);
  res += !!strcmp(str, "1");
  free(str);
  // Test %g with big number
  str = str_format("%.14g", 10000000000.0);
  res += !!strcmp(str, "10000000000");
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
  // Test %.50s with a long string
  str = str_format("%.50s",
                   "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ");
  res += !!strcmp(str, "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWX");
  free(str);
  // Test precision for %x
  str = str_format("%.8x-%.8x-%.2x", 10, 9999999, 9999999);
  res += !!strcmp(str, "0000000a-0098967f-98967f");
  free(str);
  // Test unknown specifier skip
  str = str_format("%I");
  res += !!strcmp(str, "I");
  free(str);
  // Test %s with padding
  const char *s = "Hello";
  str = str_format("[%10s]", s);
  res += !!strcmp(str, "[     Hello]");
  free(str);
  str = str_format("[%-10s]", s);
  res += !!strcmp(str, "[Hello     ]");
  free(str);
  str = str_format("[%*s]", 10, s);
  res += !!strcmp(str, "[     Hello]");
  free(str);
  str = str_format("[%-*s]", 10, s);
  res += !!strcmp(str, "[Hello     ]");
  free(str);
  // Test %p with padding
  str = str_format("%90p", &str);
  res += (strlen(str) == 90) ? 0 : 1;
  free(str);
  // Test sign prepend
  str = str_format("%+08d", 31501);
  res += !!strcmp(str, "+0031501");
  free(str);
  str = str_format("%+08d", -31501);
  res += !!strcmp(str, "-0031501");
  free(str);

  return res;
}

int test_sscanf() {
  int a, b;
  short c, d;
  float f;
  double lf;
  char str[256], str1[4];
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
  while (done == 0) {
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

  // Should wake up all threads
  pthread_t p1, p2, p3;
  pthread_cond_init(&c2, NULL);
  pthread_create(&p1, NULL, child, NULL);
  pthread_create(&p2, NULL, child, NULL);
  pthread_create(&p3, NULL, child, NULL);
  usleep(100);
  pthread_mutex_lock(&m);
  done = 1;
  pthread_cond_broadcast(&c);
  pthread_mutex_unlock(&m);
  pthread_join(p1, NULL);
  pthread_join(p2, NULL);
  pthread_join(p3, NULL);

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
    FUNC_DEF(test_errno),
    FUNC_DEF(test_getcwd_chdir),
    FUNC_DEF(test_synchronized),
#endif
    FUNC_DEF(test_qsort),
    FUNC_DEF(test_vsnprintf),
    FUNC_DEF(test_sscanf),
    FUNC_DEF(test_swscanf),
    FUNC_DEF(test_realloc),
    FUNC_DEF(test_atof),
    FUNC_DEF(test_strtof),
    FUNC_DEF(test_sem),
    FUNC_DEF(test_mtsem),
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
    FUNC_DEF(test_open),
    FUNC_DEF(test_close),
    FUNC_DEF(test_cond_var),
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
