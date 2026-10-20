/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//
//  SyncTester.h
//  TestApp
//
#import "system_headers.h"

@interface SyncTester : NSObject {
}
@property(nonatomic) int counter;
@property(nonatomic) BOOL test_ok;
- (void)recursiveSyncEnter;
- (BOOL)holdAndCheckCounter;
- (void)tryModifyCounter;
@end
