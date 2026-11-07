/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
#import "system_headers.h"

// https://developer.apple.com/documentation/uikit/uiwindow/rootviewcontroller
int test_WindowRootController(void) {
  UIWindow *window = [[UIWindow alloc] initWithFrame:CGRectMake(0, 0, 200, 300)];
  UIView *old = [[UIView alloc] initWithFrame:CGRectMake(0, 0, 10, 10)];
  [window addSubview:old];
  UIViewController *first = [UIViewController new];
  UIView *firstView = [[UIView alloc] initWithFrame:CGRectMake(0, 0, 1, 1)];
  [first setView:firstView];
  [window setRootViewController:first];
  [first release]; // The window must keep the controller alive.
  if ([window rootViewController] != first || [[window subviews] count] != 1 ||
      [[window subviews] objectAtIndex:0] != firstView || [firstView frame].size.width != 200)
    return -1;
  [window setFrame:CGRectMake(0, 0, 240, 320)];
  if ([firstView frame].size.width != 240 || [firstView frame].size.height != 320)
    return -2;
  UIViewController *second = [UIViewController new];
  UIView *secondView = [[UIView alloc] initWithFrame:CGRectMake(0, 0, 1, 1)];
  [second setView:secondView];
  [window setRootViewController:second];
  if ([window rootViewController] != second || [[window subviews] count] != 1 ||
      [[window subviews] objectAtIndex:0] != secondView)
    return -3;
  [window setRootViewController:nil];
  if ([window rootViewController] != nil || [[window subviews] count] != 0)
    return -4;
  [second release];
  [secondView release];
  [firstView release];
  [old release];
  [window release];
  return 0;
}
