/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"
#include <stdio.h>

#include "GUITestsAppDelegate.h"

int TestApp_gui_tests_main(int argc, char **argv) {
  id pool = [NSAutoreleasePool new];
  int res = UIApplicationMain(argc, argv, NULL,
                              NSStringFromClass([GUITestsAppDelegate class]));
  [pool release];
  return res;
}
