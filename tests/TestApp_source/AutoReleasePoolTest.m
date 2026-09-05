/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"
static int dealloc_counter = 0;
@implementation DeallocDetection : NSObject
- (void)dealloc {
  dealloc_counter += 1;
}
@end

int test_AutoreleasePool(void) {
  // Basic test
  {
    dealloc_counter = 0;
    NSAutoreleasePool *arp1 = [NSAutoreleasePool new];
    DeallocDetection *obj1 = [[DeallocDetection new] autorelease];
    DeallocDetection *obj2 = [[[DeallocDetection new] autorelease] retain];
    [arp1 drain];
    if (dealloc_counter != 1) {
      return -1;
    }
    if ([obj2 retainCount] != 1) {
      return -2;
    }
  }

  // Check typical autoreleasepool stack usage
  {
    dealloc_counter = 0;
    // Should not be added to autoreleasepool
    DeallocDetection *obj0 = [DeallocDetection new];
    NSAutoreleasePool *arp1 = [NSAutoreleasePool new];
    DeallocDetection *obj1 = [[DeallocDetection new] autorelease];
    NSAutoreleasePool *arp2 = [NSAutoreleasePool new];
    DeallocDetection *obj2 = [[DeallocDetection new] autorelease];
    NSAutoreleasePool *arp3 = [NSAutoreleasePool new];
    DeallocDetection *obj3 = [[DeallocDetection new] autorelease];
    [arp3 drain];
    if (dealloc_counter != 1 || [obj0 retainCount] != 1 ||
        [obj1 retainCount] != 1 || [obj2 retainCount] != 1) {
      return -3;
    }
    [arp2 drain];
    if (dealloc_counter != 2 || [obj0 retainCount] != 1 ||
        [obj1 retainCount] != 1) {
      return -4;
    }
    [arp1 drain];
    if (dealloc_counter != 3 || [obj0 retainCount] != 1) {
      return -5;
    }
    [obj0 release];
  }

  // Check atypical
  {
    dealloc_counter = 0;
    // Should not be added to autoreleasepool
    NSAutoreleasePool *arp1 = [NSAutoreleasePool new];
    DeallocDetection *obj1 = [[DeallocDetection new] autorelease];
    NSAutoreleasePool *arp2 = [NSAutoreleasePool new];
    DeallocDetection *obj2 = [[DeallocDetection new] autorelease];
    NSAutoreleasePool *arp3 = [NSAutoreleasePool new];
    DeallocDetection *obj3 = [[DeallocDetection new] autorelease];
    [arp2 drain];
    // Should dealloc both arp3 and arp2
    if (dealloc_counter != 2) {
      return -6;
    }
    // Should not dealloc arp1
    if ([obj1 retainCount] != 1) {
      return -7;
    }
    [arp1 drain];
  }
  return 0;
}
