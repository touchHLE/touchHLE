/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"
#include <pthread.h>
#include <unistd.h>

@implementation Initalize : NSObject
static bool did_init = false;
+ (void)initialize {
  [self another];
  did_init = true;
}
+ (bool)checkInitialize {
  return did_init;
}

// Make sure that we can call other messages inside initialize
+ (void)another {
}
@end

@implementation MultiThreadInitalize : NSObject
static bool mt_did_init = false;
+ (void)initialize {
  usleep(500);
  mt_did_init = true;
}

+ (bool)checkInitialize {
  return mt_did_init;
}

@end

void mt_intialize(bool *rval) {
  *rval = [MultiThreadInitalize checkInitialize];
}

int test_Initialize() {
  if (![Initalize checkInitialize]) {
    return -2;
  }

  pthread_t threads[10];
  bool rvals[10];
  for (int i = 0; i < 10; i++) {
    pthread_create(threads + i, NULL, (void *(*)(void *)) & mt_intialize,
                   rvals + i);
  }

  if (![MultiThreadInitalize checkInitialize]) {
    return -3;
  }

  bool is_ok = true;
  for (int i = 0; i < 10; i++) {
    if (pthread_join(threads[i], NULL)) {
      return -1;
    }
    is_ok &= rvals[i];
  }
  if (!is_ok) {
    return -3;
  }
  return 0;
}
