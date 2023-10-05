//
//  SyncTester.m
//  TestApp
//

#import "SyncTester.h"
#import "system_headers.h"

@implementation SyncTester : NSObject {
}
- (SyncTester *)init {
  self = [super init];
  self.counter = 0;
  self.test_ok = false;
  return self;
}
- (BOOL)holdAndCheckCounter {
  @synchronized(self) {
    self.counter = 0;
    for (int i = 0; i < 10; i++) {
      usleep(1000);
    }
    return self.counter == 0;
  }
}
- (void)tryModifyCounter {
  @synchronized(self) {
    self.counter = 1;
  }
}

- (void)recursiveSyncEnterWithCount:(int)count {
  if (count <= 0) {
    self.counter++;
    return;
  }
  @synchronized(self) {
    [self recursiveSyncEnterWithCount:(count - 1)];
    [self recursiveSyncEnterWithCount:(count - 1)];
  }
}
- (void)recursiveSyncEnter {
  self.counter = 0;
  [self recursiveSyncEnterWithCount:4];
  self.test_ok = self.counter == 16;
}
@end
