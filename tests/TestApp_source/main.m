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
#include <errno.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <CoreFoundation/CoreFoundation.h>
#include <Foundation/Foundation.h>

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
  char *str = str_format("%s %x %.3d %s", "test", 2042, 5, NULL);
  int res = strcmp(str, "test 7fa 005 (null)");
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

int test_NSString_compare() {
  if ([@"abcd" compare:@"abcd"] != NSOrderedSame)
    return -1;
  if ([@"abcd" compare:@"ABCD"] != NSOrderedDescending)
    return -1;
  if ([@"Name2.txt" compare:@"Name7.txt"
                    options:NSNumericSearch] != NSOrderedAscending)
    return -1;
  if ([@"Name7.txt" compare:@"Name25.txt"
                    options:NSNumericSearch] != NSOrderedAscending)
    return -1;
  if ([@"abc" compare:@"123" options:NSNumericSearch] != NSOrderedDescending)
    return -1;
  if ([@"abc" compare:@"abc123" options:NSNumericSearch] != NSOrderedAscending)
    return -1;
  if ([@"abc123" compare:@"abc123" options:NSNumericSearch] != NSOrderedSame)
    return -1;
  return 0;
}

int test_NSFileManager() {
  NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];
  NSString *resourcePath = [[NSBundle mainBundle] resourcePath];
  NSString *folderPath =
      [resourcePath stringByAppendingPathComponent:@"uwu_folder"];
  NSFileManager *manager = [NSFileManager defaultManager];
  NSArray *contents = [manager directoryContentsAtPath:folderPath];
  if ([contents count] != 1)
    return -1;
  NSString *first = [contents objectAtIndex:0];
  if (![first isEqualToString:@"waffle.txt"])
    return -1;
  NSString *filePath = [folderPath stringByAppendingPathComponent:first];
  BOOL isDirectory = TRUE;
  BOOL res = [manager fileExistsAtPath:filePath isDirectory:&isDirectory];
  if (!res)
    return -1;
  if (isDirectory)
    return -1;
  [pool release];
  return 0;
}

int test_chdir() {
  CFBundleRef mainBundle = CFBundleGetMainBundle();
  CFURLRef resourceURL = CFBundleCopyResourcesDirectoryURL(mainBundle);
  char path[256];
  bool res =
      CFURLGetFileSystemRepresentation(resourceURL, TRUE, (UInt8 *)path, 256);
  CFRelease(resourceURL);
  if (!res) {
    return -1;
  }
  // absolute path
  if (chdir(path) != 0) {
    return -1;
  }
  // relative path
  if (chdir("uwu_folder") != 0) {
    return -1;
  }
  FILE *file;
  file = fopen("waffle.txt", "r");
  if (file) {
    fclose(file);
    return 0;
  }
  return -1;
}

int test_eof() {
  CFBundleRef mainBundle = CFBundleGetMainBundle();
  CFURLRef fileURL =
      CFBundleCopyResourceURL(mainBundle, (CFStringRef) @"waffle.txt", NULL,
                              (CFStringRef) @"uwu_folder");
  char path[256];
  bool res =
      CFURLGetFileSystemRepresentation(fileURL, TRUE, (UInt8 *)path, 256);
  CFRelease(fileURL);
  if (!res)
    return -1;

  FILE *file;
  int c, i;
  char buf[8];

  file = fopen(path, "r");
  if (file == NULL)
    return -1;

  i = 0;
  while (true) {
    c = fgetc(file);
    if (feof(file)) {
      break;
    }
    buf[i] = c;
    i++;
  }
  buf[i] = '\0';
  fclose(file);

  return strcmp(buf, "WAFFLE\n");
}

#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
    FUNC_DEF(test_qsort),   FUNC_DEF(test_vsnprintf),
    FUNC_DEF(test_sscanf),  FUNC_DEF(test_errno),
    FUNC_DEF(test_realloc), FUNC_DEF(test_NSString_compare),
    FUNC_DEF(test_chdir),   FUNC_DEF(test_NSFileManager),
    FUNC_DEF(test_eof),
};

int main(int argc, char *argv[]) {
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
  return tests_run == tests_passed ? 0 : 1;
}
