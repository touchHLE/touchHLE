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
}
+ (bool)checkInitialize {
  return did_init;
}

// Make sure that we can call other messages inside initialize
+ (void)another {
  did_init = true;
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

@implementation SuperInitialize : NSObject
static bool super_did_init = false;
+ initialize {
  super_did_init = true;
}
+ (bool)checkInitialize {
  return super_did_init;
}
@end

@implementation Sub1Initialize : SuperInitialize
static bool sub1_did_init = false;
+ initialize {
  sub1_did_init = true;
}
+ (bool)checkInitialize {
  return [super checkInitialize] && sub1_did_init;
}
@end

@implementation Sub2Initialize : SuperInitialize
static bool sub2_did_init = false;
+ initialize {
  sub2_did_init = true;
}
+ (bool)checkInitialize {
  return [super checkInitialize] && sub2_did_init;
}
@end

// The behaviour for this is kinda unintuitive: (From
// https://developer.apple.com/documentation/objectivec/nsobject-swift.class/initialize()?language=objc)
// "The superclass implementation may be called multiple times if subclasses do
// not implement initialize—the runtime will call the inherited
// implementation—or if subclasses explicitly call [super initialize]. If you
// want to protect yourself from being run multiple times, you can structure
// your implementation along these lines (guard implementation)"
@implementation GuardInitialize : NSObject
static int init_count = 0;
static int guarded_init_count = 0;
+ initialize {
  init_count++;
  if (self == [GuardInitialize class]) {
    guarded_init_count++;
  }
}
+ (bool)checkInitialize:(int)count {
  return init_count == count && guarded_init_count == 1;
}
@end

@implementation Sub1GuardInitialize : GuardInitialize
// This one does not have a +initialize, so it should bump init_count but not
// guarded_init_count
@end

@implementation Sub2GuardInitialize : GuardInitialize
// This one does have a +initialize, so it should not bump init_count nor
// guarded_init_count
+ initialize {
}
@end

void mt_intialize(bool *rval) {
  *rval = [MultiThreadInitalize checkInitialize];
}

int test_Initialize() {
  // Regular +initialize test
  if (did_init) {
    return -20;
  }
  if (![Initalize checkInitialize]) {
    return -21;
  }

  // Multithreaded +initialize test
  if (mt_did_init) {
    return -22;
  }
  pthread_t threads[10];
  bool rvals[10];
  for (int i = 0; i < 10; i++) {
    pthread_create(threads + i, NULL, (void *(*)(void *)) & mt_intialize,
                   rvals + i);
  }

  if (![MultiThreadInitalize checkInitialize]) {
    return -30;
  }

  bool is_ok = true;
  for (int i = 0; i < 10; i++) {
    if (pthread_join(threads[i], NULL)) {
      return -1;
    }
    is_ok &= rvals[i];
  }
  if (!is_ok) {
    return -31;
  }

  // Superclass initialize test
  if (super_did_init || sub1_did_init || sub1_did_init) {
    return -40;
  }
  if (![Sub1Initialize checkInitialize]) {
    return -41;
  }
  if (sub2_did_init) {
    return -42;
  }
  if (![Sub2Initialize checkInitialize]) {
    return -43;
  }

  // Guard initialize test
  if (init_count != 0 && guarded_init_count != 0) {
    return -50;
  }
  if (![GuardInitialize checkInitialize:1]) {
    return -51;
  }
  if (![Sub1GuardInitialize checkInitialize:2]) {
    return -52;
  }
  if (![Sub2GuardInitialize checkInitialize:2]) {
    return -53;
  }

  return 0;
}
