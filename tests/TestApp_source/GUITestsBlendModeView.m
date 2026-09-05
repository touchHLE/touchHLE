/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"

#include "GUITestsAppDelegate.h"
#include "GUITestsBlendModeView.h"

#define NUM_TESTS 7

// Filled with a yellow base rectangle and a cyan overlay rectangle on a gray
// background. The blend mode chosen for the overlay determines the visible
// color where the two rectangles intersect. With base = (1, 1, 0) and source =
// (0, 1, 1):
//   - kCGBlendModeNormal:   overlap = source         = (0, 1, 1) cyan
//   - kCGBlendModeMultiply: overlap = base * source  = (0, 1, 0) green
//   - kCGBlendModeScreen:   overlap = 1-(1-b)*(1-s)  = (1, 1, 1) white
//   - kCGBlendModeDarken:   overlap = min(b, s)      = (0, 1, 0) green
//   - kCGBlendModeLighten:  overlap = max(b, s)      = (1, 1, 1) white
//
// When `clearBackdropTest` is set, the gray background fill and the yellow
// base rectangle are skipped, and the top half of the view is filled with an
// opaque 70% gray via `drawRect:` (going through `blend_premultiplied` with a
// backdrop alpha of 0, because `clearsContextBeforeDrawing` has just zeroed
// the bitmap). The bottom half is left untouched so the cleared (transparent)
// pixels let the layer's GL-rendered backgroundColor show through directly.
// When `midToneTest` is set, the gray background fill is replaced with black
// and both the base and overlay rectangles are drawn in opaque sRGB 0.7 gray,
// so the configured blend mode is exercised on mid-tone (non-{0,1}) channels.
@interface GUITestsBlendModeTestArea : UIView {
@public
  CGBlendMode blendMode;
  BOOL clearBackdropTest;
  BOOL midToneTest;
}
@end

@implementation GUITestsBlendModeTestArea : UIView
- (void)drawRect:(CGRect)rect {
  CGContextRef context = UIGraphicsGetCurrentContext();
  CGRect bounds = [self bounds];

  if (clearBackdropTest) {
    // Fill ONLY the top half with the same opaque 70% gray as the layer's
    // backgroundColor. The bottom half stays untouched so the cleared bitmap
    // lets the backgroundColor show through unchanged.
    CGContextSetBlendMode(context, blendMode);
    CGContextSetRGBFillColor(context, 0.7, 0.7, 0.7, 1.0);
    CGContextFillRect(
        context, CGRectMake(0, 0, bounds.size.width, bounds.size.height / 2));
    CGContextSetBlendMode(context, kCGBlendModeNormal);
    return;
  }

  // Background fill: gray, so the surrounding test area color is visible too.
  // For the mid-tone test, use black so the overlap region (lighter than the
  // base) is clearly distinguishable.
  if (midToneTest) {
    CGContextSetRGBFillColor(context, 0.0, 0.0, 0.0, 1.0);
  } else {
    CGContextSetRGBFillColor(context, 0.5, 0.5, 0.5, 1.0);
  }
  CGContextFillRect(context, bounds);

  // First (base) rectangle: opaque yellow in the upper-left (or sRGB 0.7 gray
  // for the mid-tone test — chosen so that two such values screen-blended in
  // linear space exceed 1.0 and force a clamp on a buggy implementation).
  CGContextSetBlendMode(context, kCGBlendModeNormal);
  if (midToneTest) {
    CGContextSetRGBFillColor(context, 0.7, 0.7, 0.7, 1.0);
  } else {
    CGContextSetRGBFillColor(context, 1.0, 1.0, 0.0, 1.0);
  }
  CGContextFillRect(context, CGRectMake(40, 40, 180, 180));

  // Second (overlay) rectangle: opaque cyan (or the same sRGB 0.7 gray for
  // the mid-tone test), shifted so it overlaps the base rectangle. The blend
  // mode under test controls how the overlap composites.
  CGContextSetBlendMode(context, blendMode);
  if (midToneTest) {
    CGContextSetRGBFillColor(context, 0.7, 0.7, 0.7, 1.0);
  } else {
    CGContextSetRGBFillColor(context, 0.0, 1.0, 1.0, 1.0);
  }
  CGContextFillRect(context, CGRectMake(120, 120, 180, 180));

  // Restore the default blend mode so later drawing isn't affected.
  CGContextSetBlendMode(context, kCGBlendModeNormal);
}
@end

@implementation GUITestsBlendModeView : UIView

UILabel *blendTitle;
UILabel *expectationLabel;
GUITestsBlendModeTestArea *blendTestArea;
NSUInteger blendTestNum;

- (instancetype)initWithFrame:(CGRect)frame {
  [super initWithFrame:frame];

  blendTitle = [[UILabel alloc] initWithFrame:CGRectMake(0, 0, 320, 20)];
  blendTitle.text =
      [NSString stringWithUTF8String:"CGContextSetBlendMode (press →)"];
  blendTitle.textAlignment = UITextAlignmentCenter;
  [self addSubview:blendTitle];

  expectationLabel =
      [[UILabel alloc] initWithFrame:CGRectMake(0, 380, 320, 20)];
  expectationLabel.textAlignment = UITextAlignmentCenter;
  expectationLabel.textColor = [UIColor whiteColor];
  expectationLabel.backgroundColor = [UIColor clearColor];
  [self addSubview:expectationLabel];

  UIButton *button1 = [UIButton buttonWithType:UIButtonTypeRoundedRect];
  [button1 setTitle:[NSString stringWithUTF8String:"←"]
           forState:UIControlStateNormal];
  [button1 setFrame:CGRectMake(0, 420, 40, 40)];
  [button1 addTarget:self
                action:@selector(prevTest)
      forControlEvents:UIControlEventTouchUpInside];
  [self addSubview:button1];
  [button1 layoutSubviews]; // FIXME: workaround for touchHLE not calling this

  UIButton *button2 = [UIButton buttonWithType:UIButtonTypeRoundedRect];
  [button2 setTitle:[NSString stringWithUTF8String:"→"]
           forState:UIControlStateNormal];
  [button2 setFrame:CGRectMake(280, 420, 40, 40)];
  [button2 addTarget:self
                action:@selector(nextTest)
      forControlEvents:UIControlEventTouchUpInside];
  [self addSubview:button2];
  [button2 layoutSubviews]; // FIXME: workaround for touchHLE not calling this

  blendTestNum = 0;

  return self;
}

- (void)dealloc {
  [blendTitle release];
  [expectationLabel release];
  [blendTestArea release];
  [super dealloc];
}

- (void)prevTest {
  if (blendTestNum > 1)
    blendTestNum--;
  [self displayTest];
}

- (void)nextTest {
  if (blendTestNum < NUM_TESTS)
    blendTestNum++;
  [self displayTest];
}

- (void)displayTest {
  blendTitle.text = [NSString
      stringWithFormat:[NSString stringWithUTF8String:"BlendMode test %u/%u"],
                       blendTestNum, NUM_TESTS];
  [blendTestArea removeFromSuperview];
  [blendTestArea release];
  blendTestArea = [[GUITestsBlendModeTestArea alloc]
      initWithFrame:CGRectMake(10, 30, 300, 340)];
  blendTestArea.backgroundColor = [UIColor blackColor];
  [self addSubview:blendTestArea];
  [blendTestArea setNeedsDisplay]; // FIXME: normally we should not need that...

  [self performSelector:NSSelectorFromString([NSString
                            stringWithFormat:[NSString
                                                 stringWithUTF8String:"test%u"],
                                             blendTestNum])];
}

// Test 1: kCGBlendModeNormal. The cyan overlay should fully replace the yellow
// rectangle where the two rectangles intersect.
- (void)test1 {
  blendTestArea->blendMode = kCGBlendModeNormal;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Normal: overlap is cyan"];
}

// Test 2: kCGBlendModeMultiply. Where the rectangles overlap, the colors
// multiply: (1, 1, 0) * (0, 1, 1) = (0, 1, 0), i.e. green.
- (void)test2 {
  blendTestArea->blendMode = kCGBlendModeMultiply;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Multiply: overlap is green"];
}

// Test 3: kCGBlendModeScreen. Per-channel screen blend:
// 1 - (1 - base) * (1 - src) = 1 - (1-1,1-1,1-0)*(1-0,1-1,1-1)
// = (1, 1, 1), i.e. white.
- (void)test3 {
  blendTestArea->blendMode = kCGBlendModeScreen;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Screen: overlap is white"];
}

// Test 4: kCGBlendModeDarken. Per-channel min(base, src):
// min((1, 1, 0), (0, 1, 1)) = (0, 1, 0), i.e. green.
- (void)test4 {
  blendTestArea->blendMode = kCGBlendModeDarken;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Darken: overlap is green"];
}

// Test 5: kCGBlendModeLighten. Per-channel max(base, src):
// max((1, 1, 0), (0, 1, 1)) = (1, 1, 1), i.e. white.
- (void)test5 {
  blendTestArea->blendMode = kCGBlendModeLighten;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Lighten: overlap is white"];
}

// Test 6: kCGBlendModeNormal into a context whose backdrop alpha is 0. The
// top half of the test area is filled with an opaque 70% gray via drawRect:
// (going through blend_premultiplied with αb = 0), and the bottom half is
// left untouched so the layer's GL-rendered backgroundColor - set to the
// same 70% gray here - shows through.
- (void)test6 {
  blendTestArea.backgroundColor = [UIColor colorWithRed:0.7
                                                  green:0.7
                                                   blue:0.7
                                                  alpha:1.0];
  blendTestArea->blendMode = kCGBlendModeNormal;
  blendTestArea->clearBackdropTest = YES;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Normal: uniform 70% gray (no stripe)"];
}

// Test 7: kCGBlendModeScreen with mid-tone colors. The overlap should be
// a clearly lighter shade than the sRGB 0.7 base/overlay, but distinctly
// NOT pure white.
- (void)test7 {
  blendTestArea->blendMode = kCGBlendModeScreen;
  blendTestArea->midToneTest = YES;
  expectationLabel.text =
      [NSString stringWithUTF8String:"Screen: overlap is light gray "
                                     "(not white)"];
}

@end
