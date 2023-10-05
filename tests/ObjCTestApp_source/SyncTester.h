//
//  SyncTester.h
//  TestApp
//

#import "system_headers.h"

@interface SyncTester : NSObject {
}

@property (nonatomic) int counter;
@property (nonatomic) BOOL test_ok;

- (void) recursiveSyncEnter;
- (BOOL) holdAndCheckCounter;
- (void) tryModifyCounter;
@end
