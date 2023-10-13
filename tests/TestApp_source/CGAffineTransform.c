/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Tests related to CGAffineTransform.
// These are in their own file so you can easily compile them separately and
// run them on macOS, with a command like:
//
// cc tests/TestApp_source/CGAffineTransform.c -framework CoreGraphics
// -DDEFINE_ME_WHEN_BUILDING_ON_MACOS -Dtest_CGAffineTransform=main && ./a.out;
// echo $?

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
  CGFloat x, y;
} CGPoint;
bool CGPointEqualToPoint(CGPoint, CGPoint);
typedef struct {
  CGFloat width, height;
} CGSize;
bool CGSizeEqualToSize(CGSize, CGSize);
typedef struct {
  CGPoint origin;
  CGSize size;
} CGRect;
bool CGRectEqualToRect(CGRect, CGRect);

typedef struct {
  CGFloat a, b, c, d, tx, ty;
} CGAffineTransform;
// extern const CGAffineTransform CGAffineTransformIdentity;
bool CGAffineTransformIsIdentity(CGAffineTransform);
bool CGAffineTransformEqualToTransform(CGAffineTransform, CGAffineTransform);
CGAffineTransform CGAffineTransformMake(CGFloat, CGFloat, CGFloat, CGFloat,
                                        CGFloat, CGFloat);
CGAffineTransform CGAffineTransformMakeRotation(CGFloat);
CGAffineTransform CGAffineTransformMakeScale(CGFloat, CGFloat);
CGAffineTransform CGAffineTransformMakeTranslation(CGFloat, CGFloat);
CGAffineTransform CGAffineTransformConcat(CGAffineTransform, CGAffineTransform);
CGAffineTransform CGAffineTransformRotate(CGAffineTransform, CGFloat);
CGAffineTransform CGAffineTransformScale(CGAffineTransform, CGFloat, CGFloat);
CGAffineTransform CGAffineTransformTranslate(CGAffineTransform, CGFloat,
                                             CGFloat);
CGPoint CGPointApplyAffineTransform(CGPoint, CGAffineTransform);
CGSize CGSizeApplyAffineTransform(CGSize, CGAffineTransform);
CGRect CGRectApplyAffineTransform(CGRect, CGAffineTransform);

// Debugging code:
int printf(const char *, ...);
void dump_transform(CGAffineTransform t) {
  printf(".a: %f\n", t.a);
  printf(".b: %f\n", t.b);
  printf(".c: %f\n", t.c);
  printf(".d: %f\n", t.d);
  printf(".tx: %f\n", t.tx);
  printf(".ty: %f\n", t.ty);
}

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

  CGAffineTransform nonsense = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0};
  success = success && !CGAffineTransformEqualToTransform(
                           identity_from_initializer, nonsense);
  success = success && !CGAffineTransformEqualToTransform(
                           nonsense, identity_from_initializer);
  success = success && CGAffineTransformEqualToTransform(nonsense, nonsense);
  success =
      success && CGAffineTransformEqualToTransform(identity_from_initializer,
                                                   identity_from_initializer);

  CGAffineTransform nonsense_from_make =
      CGAffineTransformMake(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
  success = success &&
            CGAffineTransformEqualToTransform(nonsense, nonsense_from_make);
  success = success && !CGAffineTransformEqualToTransform(
                           identity_from_initializer, nonsense_from_make);
  success = success && CGAffineTransformEqualToTransform(
                           nonsense_from_make,
                           CGAffineTransformMake(1.0, 2.0, 3.0, 4.0, 5.0, 6.0));

  success = success &&
            CGAffineTransformIsIdentity(CGAffineTransformMakeRotation(0.0));
  // Further testing rotation is tricky due to floating point imprecision and
  // the fact CGAffineTransformMakeRotation() rotates in the
  // _opposite direction_ on macOS for some reason, so it's not done here.

  success = success &&
            CGAffineTransformIsIdentity(CGAffineTransformMakeScale(1.0, 1.0));
  success = success && CGAffineTransformEqualToTransform(
                           CGAffineTransformMakeScale(2.0, 3.0),
                           CGAffineTransformMake(2.0, 0.0, 0.0, 3.0, 0.0, 0.0));

  success = success && CGAffineTransformIsIdentity(
                           CGAffineTransformMakeTranslation(0.0, 0.0));
  success = success && CGAffineTransformEqualToTransform(
                           CGAffineTransformMakeTranslation(2.0, 3.0),
                           CGAffineTransformMake(1.0, 0.0, 0.0, 1.0, 2.0, 3.0));

  success =
      success && CGAffineTransformIsIdentity(CGAffineTransformConcat(
                     identity_from_initializer, identity_from_initializer));
  success = success &&
            CGAffineTransformEqualToTransform(
                CGAffineTransformConcat(identity_from_initializer, nonsense),
                nonsense);
  success = success &&
            CGAffineTransformEqualToTransform(
                CGAffineTransformConcat(nonsense, identity_from_initializer),
                nonsense);
  success =
      success &&
      CGAffineTransformEqualToTransform(
          CGAffineTransformConcat(CGAffineTransformMakeTranslation(2.0, 0.0),
                                  CGAffineTransformMakeTranslation(0.0, 3.0)),
          CGAffineTransformMakeTranslation(2.0, 3.0));
  success = success && CGAffineTransformEqualToTransform(
                           CGAffineTransformConcat(
                               CGAffineTransformMakeScale(-1.0, -1.0),
                               CGAffineTransformConcat(
                                   CGAffineTransformMakeTranslation(2.0, 3.0),
                                   CGAffineTransformMakeScale(-1.0, -1.0))),
                           CGAffineTransformMakeTranslation(-2.0, -3.0));

  success =
      success && CGAffineTransformEqualToTransform(
                     CGAffineTransformMakeRotation(1.0),
                     CGAffineTransformRotate(identity_from_initializer, 1.0));
  success = success &&
            CGAffineTransformEqualToTransform(
                CGAffineTransformMakeScale(2.0, 3.0),
                CGAffineTransformScale(identity_from_initializer, 2.0, 3.0));
  success = success && CGAffineTransformEqualToTransform(
                           CGAffineTransformMakeTranslation(2.0, 3.0),
                           CGAffineTransformTranslate(identity_from_initializer,
                                                      2.0, 3.0));

  success = success &&
            CGPointEqualToPoint((CGPoint){-2.0, 6.0},
                                CGPointApplyAffineTransform(
                                    (CGPoint){2.0, 3.0},
                                    CGAffineTransformMakeScale(-1.0, 2.0)));

  success =
      success && CGSizeEqualToSize((CGSize){-2.0, 6.0},
                                   CGSizeApplyAffineTransform(
                                       (CGSize){2.0, 3.0},
                                       CGAffineTransformMakeScale(-1.0, 2.0)));

  // Non-rectangle-preserving transforms are more complicated, not tested here.
  success =
      success && CGRectEqualToRect((CGRect){4.0, 6.0, 2.0, 4.0},
                                   CGRectApplyAffineTransform(
                                       (CGRect){2.0, 3.0, 1.0, 2.0},
                                       CGAffineTransformMakeScale(2.0, 2.0)));
  success =
      success && CGRectEqualToRect((CGRect){-6.0, -10.0, 2.0, 4.0},
                                   CGRectApplyAffineTransform(
                                       (CGRect){2.0, 3.0, 1.0, 2.0},
                                       CGAffineTransformMakeScale(-2.0, -2.0)));

  return !success;
}
