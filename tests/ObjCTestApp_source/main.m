/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This is a main file for the TestApp which is used for integration testing.
// See also tests/README.md and tests/integration.rs for the details of how it
// is compiled and run.

#import "system_headers.h"
#include "SyncTester.h"
// === Main code ===

typedef struct {
    SyncTester* tester;
    BOOL res;
} sync_test_arg;

void* modify(sync_test_arg* arg) {
    SyncTester* tester = arg->tester;
    arg->res = [tester holdAndCheckCounter];
    return NULL;
}

void* try_modify(SyncTester* tester) {
    [tester tryModifyCounter];
    return NULL;

}

int test_synchronized(){
        SyncTester *sync_test = [SyncTester new];
        sync_test_arg* arg = malloc(sizeof(sync_test_arg));
        memset(arg, 0, sizeof(sync_test_arg));
        arg->tester = sync_test;
        pthread_t locking_thread;
        pthread_create(&locking_thread, NULL, (void* (*)(void*)) & modify, arg);

        pthread_t blocked_threads[10];
        for (int i = 0; i < 10; i++){
            pthread_create(blocked_threads + i, NULL, (void*(*)(void*))&try_modify, sync_test);
        }
        if (pthread_join(locking_thread, NULL))
            return -1;
        if (!arg->res) return -1;
        [sync_test recursiveSyncEnter];
        if(!sync_test.test_ok)
            return -1;
        return 0;
}
#define FUNC_DEF(func)                                                         \
  { &func, #func }
struct {
  int (*func)();
  const char *name;
} test_func_array[] = {
    FUNC_DEF(test_synchronized)
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
