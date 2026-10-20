/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"

#include "GUITestsAppDelegate.h"
#include "GUITestsMainMenu.h"

@implementation GUITestsAppDelegate : NSObject
UIWindow *window;
UIView *mainView;

- (void)applicationDidFinishLaunching:(id)app {
  NSAutoreleasePool *pool = [NSAutoreleasePool new];
  window =
      [[UIWindow alloc] initWithFrame:[[UIScreen mainScreen] applicationFrame]];
  [self setMainView:[[[GUITestsMainMenu alloc] initWithFrame:[window bounds]]
                        autorelease]];
  [pool drain];
  [window makeKeyAndVisible];

  [NSTimer scheduledTimerWithTimeInterval:(1.0 / 60.0)
                                   target:self
                                 selector:@selector(onTick:)
                                 userInfo:nil
                                  repeats:YES];
}

// This is a clumsy way to swap out views to switch between different sections
// of the app. A proper implementation would probably involve UIViewController,
// but touchHLE's implementation of it is quite incomplete. This'll do for now.
- (void)setMainView:(UIView *)view {
  [view retain];
  [mainView removeFromSuperview];
  [mainView release];
  mainView = view;
  [window addSubview:view];
}

- (void)onTick:(NSTimer *)timer {
  if ([mainView respondsToSelector:@selector(tick)]) {
    [mainView performSelector:@selector(tick)];
  }
}

- (void)dealloc {
  [mainView release];
  [window release];
  [super dealloc];
}
@end
