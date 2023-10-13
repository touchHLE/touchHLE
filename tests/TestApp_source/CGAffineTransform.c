/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Tests related to CGAffineTransform.
// These are in their own file so you can easily compile them separately and
// run them on macOS, like this:
//
// $ cat > main.c
//  #include <stdio.h>
//  int test_CGAffineTransform(void);
//  int main(void)
//  {
//    puts(test_CGAffineTransform() ? "failed" : "passed");
//  }
// $ cc tests/TestApp_source/CGAffineTransform.c main.c -framework CoreGraphics -DDEFINE_ME_WHEN_BUILDING_ON_MACOS && ./a.out

// === Declarations ===

// <stdbool.h>
typedef _Bool bool;

// Stuff from various Core Graphics headers.

#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
typedef double CGFloat; // 64-bit definition (not supported by touchHLE)
#else
typedef float CGFloat;
#endif

typedef struct {
  CGFloat a, b, c, d, tx, ty;
} CGAffineTransform;
// extern const CGAffineTransform CGAffineTransformIdentity;
bool CGAffineTransformIsIdentity(CGAffineTransform);

// Debugging code:
/*int printf(const char *, ...);
void dump_transform(CGAffineTransform t) {
  printf(".a: %f\n", t.a);
  printf(".b: %f\n", t.b);
  printf(".c: %f\n", t.c);
  printf(".d: %f\n", t.d);
  printf(".tx: %f\n", t.tx);
  printf(".ty: %f\n", t.ty);
}*/

// === Main code ===

int test_CGAffineTransform(void) {
  bool success = 1;

  // TODO: test CGAffineTransformIdentity. It seems like non-lazy symbols (i.e.
  // non-function symbols) are not linked correctly, probably due to one of the
  // cursed things done to build TestApp, so it can't be tested right now.
  /*CGAffineTransform identity_from_constant = CGAffineTransformIdentity;
  success = success && CGAffineTransformIsIdentity(identity_from_constant);*/

  CGAffineTransform identity_from_initializer = {
      .a = 1.0,
      .b = 0.0,
      .c = 0.0,
      .d = 1.0,
      .tx = 0.0,
      .ty = 0.0,
  };
  success = success && CGAffineTransformIsIdentity(identity_from_initializer);

  return !success;
}
