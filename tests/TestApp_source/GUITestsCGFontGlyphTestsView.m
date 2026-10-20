/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"
#include <math.h>

#include "GUITestsCGFontGlyphTestsView.h"

#define NUM_TESTS 8

#define BITMAP_WIDTH 280
#define BITMAP_HEIGHT 160
#define BITMAP_BYTES_PER_ROW (BITMAP_WIDTH * 4)

@implementation GUITestsCGFontGlyphTestsView : UIView

UILabel *fontTitle;
UIImageView *bitmapView;
UILabel *summaryLabel;
UILabel *paramsLabel1;
UILabel *paramsLabel2;
UILabel *paramsLabel3;
CGFontRef testFont;
NSUInteger fontTestNum;

- (instancetype)initWithFrame:(CGRect)frame {
  [super initWithFrame:frame];

  fontTitle = [[UILabel alloc] initWithFrame:CGRectMake(0, 0, 320, 20)];
  fontTitle.text =
      [NSString stringWithUTF8String:"CGFont/CGGlyph tests (press →)"];
  fontTitle.textAlignment = UITextAlignmentCenter;
  [self addSubview:fontTitle];

  bitmapView = [[UIImageView alloc]
      initWithFrame:CGRectMake(20, 24, BITMAP_WIDTH, BITMAP_HEIGHT)];
  bitmapView.backgroundColor = [UIColor darkGrayColor];
  [self addSubview:bitmapView];

  summaryLabel = [[UILabel alloc] initWithFrame:CGRectMake(10, 188, 300, 40)];
  summaryLabel.textColor = [UIColor whiteColor];
  summaryLabel.backgroundColor = [UIColor clearColor];
  [summaryLabel setNumberOfLines:0];
  [self addSubview:summaryLabel];

  paramsLabel1 = [[UILabel alloc] initWithFrame:CGRectMake(10, 230, 300, 40)];
  paramsLabel1.textColor = [UIColor whiteColor];
  paramsLabel1.backgroundColor = [UIColor clearColor];
  [paramsLabel1 setNumberOfLines:0];
  [self addSubview:paramsLabel1];

  paramsLabel2 = [[UILabel alloc] initWithFrame:CGRectMake(10, 272, 300, 40)];
  paramsLabel2.textColor = [UIColor whiteColor];
  paramsLabel2.backgroundColor = [UIColor clearColor];
  [paramsLabel2 setNumberOfLines:0];
  [self addSubview:paramsLabel2];

  paramsLabel3 = [[UILabel alloc] initWithFrame:CGRectMake(10, 314, 300, 40)];
  paramsLabel3.textColor = [UIColor whiteColor];
  paramsLabel3.backgroundColor = [UIColor clearColor];
  [paramsLabel3 setNumberOfLines:0];
  [self addSubview:paramsLabel3];

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

  fontTestNum = 0;

  return self;
}

- (void)dealloc {
  [fontTitle release];
  [bitmapView release];
  [summaryLabel release];
  [paramsLabel1 release];
  [paramsLabel2 release];
  [paramsLabel3 release];
  CGFontRelease(testFont);
  [super dealloc];
}

// Lazy-initialised LiberationMono-Regular CGFontRef, shared across all tests.
- (CGFontRef)testFont {
  if (!testFont) {
    CFStringRef fontName =
        (CFStringRef)[NSString stringWithUTF8String:"LiberationMono-Regular"];
    CFStringRef fontExt = (CFStringRef)[NSString stringWithUTF8String:"ttf"];
    CFBundleRef mainBundle = CFBundleGetMainBundle();
    CFURLRef url = CFBundleCopyResourceURL(mainBundle, fontName, fontExt, NULL);
    CFDataRef data = (CFDataRef)[NSData dataWithContentsOfURL:url];
    CGDataProviderRef provider = CGDataProviderCreateWithCFData(data);
    testFont = CGFontCreateWithDataProvider(provider);
    CFRelease(provider);
    CFRelease(url);
  }
  return testFont;
}

// Look up a single glyph by its character via CGFontGetGlyphsForUnichars in
// the shared font. CGContextShowGlyphsAtPoint takes raw CGGlyph indices,
// which are font-specific.
- (CGGlyph)glyphForChar:(UniChar)c {
  CGGlyph g = 0;
  CGFontGetGlyphsForUnichars([self testFont], &c, &g, 1);
  return g;
}

- (void)prevTest {
  if (fontTestNum > 1)
    fontTestNum--;
  [self displayTest];
}
- (void)nextTest {
  if (fontTestNum < NUM_TESTS)
    fontTestNum++;
  [self displayTest];
}
- (void)displayTest {
  fontTitle.text = [NSString
      stringWithFormat:[NSString
                           stringWithUTF8String:"CGFont/CGGlyph test %u/%u"],
                       fontTestNum, NUM_TESTS];
  summaryLabel.text = [NSString stringWithUTF8String:""];
  paramsLabel1.text = [NSString stringWithUTF8String:""];
  paramsLabel2.text = [NSString stringWithUTF8String:""];
  paramsLabel3.text = [NSString stringWithUTF8String:""];
  [bitmapView setImage:nil];

  if (fontTestNum == 0) {
    return;
  }

  [self performSelector:NSSelectorFromString([NSString
                            stringWithFormat:[NSString
                                                 stringWithUTF8String:"test%u"],
                                             fontTestNum])];
}

// Allocate a fresh RGBA CGBitmapContext primed with a white background and
// black fill colour, ready for a CGContextShowGlyphsAtPoint call. Uses CG's
// native coordinate system - y=0 at the lower-left - so glyphs render upright
// out of the box.
- (CGContextRef)makeContext {
  CGColorSpaceRef colorSpace = CGColorSpaceCreateDeviceRGB();
  CGContextRef context = CGBitmapContextCreate(
      NULL, (size_t)BITMAP_WIDTH, (size_t)BITMAP_HEIGHT, 8,
      (size_t)BITMAP_BYTES_PER_ROW, colorSpace, kCGImageAlphaPremultipliedLast);
  CGColorSpaceRelease(colorSpace);

  CGContextSetRGBFillColor(context, 1.0, 1.0, 1.0, 1.0);
  CGContextFillRect(context, CGRectMake(0.0, 0.0, BITMAP_WIDTH, BITMAP_HEIGHT));
  CGContextSetRGBFillColor(context, 0.0, 0.0, 0.0, 1.0);
  return context;
}

- (void)presentContext:(CGContextRef)context {
  CGImageRef cgImage = CGBitmapContextCreateImage(context);
  UIImage *image = [UIImage imageWithCGImage:cgImage];
  [bitmapView setImage:image];
  CGImageRelease(cgImage);
  CGContextRelease(context);
}

// Build a "G0,G1,G2,..." string from a CGGlyph array. Used by the tests to
// reveal the exact glyph indices passed to CGContextShowGlyphsAtPoint, which
// is the most common thing to second-guess when the rendered output looks
// wrong.
- (NSString *)describeGlyphs:(const CGGlyph *)glyphs count:(size_t)count {
  NSString *result = [NSString stringWithUTF8String:"glyphs={"];
  for (size_t i = 0; i < count; i++) {
    NSString *sep = (i == 0) ? [NSString stringWithUTF8String:""]
                             : [NSString stringWithUTF8String:","];
    result =
        [NSString stringWithFormat:[NSString stringWithUTF8String:"%@%@%u"],
                                   result, sep, (unsigned)glyphs[i]];
  }
  result =
      [NSString stringWithFormat:[NSString stringWithUTF8String:"%@}"], result];
  return result;
}

// Test 1: the smallest possible CGContextShowGlyphsAtPoint call. One font set
// via CGContextSetFont + CGContextSetFontSize, one short ASCII-glyph run,
// default text matrix. If this draws "Hello" the basic plumbing of
// CGContextShowGlyphsAtPoint is working.
- (void)test1 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 24.0);

  CGGlyph glyphs[] = {[self glyphForChar:'H'], [self glyphForChar:'e'],
                      [self glyphForChar:'l'], [self glyphForChar:'l'],
                      [self glyphForChar:'o']};
  size_t count = sizeof(glyphs) / sizeof(glyphs[0]);
  CGContextShowGlyphsAtPoint(context, 20.0, 80.0, glyphs, count);

  summaryLabel.text = [NSString
      stringWithUTF8String:"test1: CGContextShowGlyphsAtPoint Hello@(20,80)"];
  paramsLabel1.text = [NSString
      stringWithUTF8String:
          "font: LiberationMono-Regular via CGContextSetFont, size 24"];
  paramsLabel2.text = [self describeGlyphs:glyphs count:count];
  paramsLabel3.text =
      [NSString stringWithUTF8String:"fill=(0,0,0,1) bg=(1,1,1,1)"];

  NSLog([NSString stringWithUTF8String:
                      "CGFont/CGGlyph test1: showing \"Hello\" (5 glyphs)"]);

  [self presentContext:context];
}

// Test 2: sweep CGContextSetFontSize against one CGContextSetFont. The Ag
// pairs should grow visibly left-to-right; if they overlap the host's metrics
// reporting is broken.
- (void)test2 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);

  CGGlyph glyphs[] = {[self glyphForChar:'A'], [self glyphForChar:'g']};
  size_t count = sizeof(glyphs) / sizeof(glyphs[0]);
  CGFloat sizes[] = {10.0, 14.0, 20.0, 28.0, 40.0};
  CGFloat x = 10.0;
  for (size_t i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
    CGContextSetFontSize(context, sizes[i]);
    CGContextShowGlyphsAtPoint(context, x, 100.0, glyphs, count);
    x += sizes[i] * 1.6;
  }

  summaryLabel.text =
      [NSString stringWithUTF8String:"test2: \"Ag\" at sizes 10/14/20/28/40"];
  paramsLabel1.text =
      [NSString stringWithUTF8String:"baseline y=100, font size set per pass"];
  paramsLabel2.text = [self describeGlyphs:glyphs count:count];
  paramsLabel3.text = [NSString
      stringWithUTF8String:"x advance approximation: x += size * 1.6"];

  NSLog([NSString stringWithUTF8String:"CGFont/CGGlyph test2: 5 size passes"]);

  [self presentContext:context];
}

// Test 3: every glyph index 0..255 in a 16x16 grid. A blanket call to
// CGContextShowGlyphsAtPoint, one glyph at a time, makes missing glyphs
// jump out.
- (void)test3 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 8.0);

  for (int row = 0; row < 16; row++) {
    for (int col = 0; col < 16; col++) {
      CGFloat x = 4.0 + 16.0 * (CGFloat)col;
      CGFloat y = 12.0 + 9.0 * (CGFloat)row;
      CGGlyph one = (CGGlyph)(row * 16 + col);
      CGContextShowGlyphsAtPoint(context, x, y, &one, 1);
    }
  }

  summaryLabel.text = [NSString
      stringWithUTF8String:"test3: glyph indices 0..255, font size 8pt"];
  paramsLabel1.text = [NSString
      stringWithUTF8String:"grid: 16x16, cellW=16 cellH=9, origin (4,12)"];
  paramsLabel2.text = [NSString
      stringWithUTF8String:"runs: 256 single-glyph CGContextShowGlyphsAtPoint"];

  NSLog([NSString
      stringWithUTF8String:
          "CGFont/CGGlyph test3: 256 single-glyph runs in 16x16 grid"]);

  [self presentContext:context];
}

// Test 4: color and alpha. Three translucent overlapping '#' glyphs plus one
// fully opaque, all via CGContextShowGlyphsAtPoint. Surfaces text-path
// blending bugs.
- (void)test4 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 48.0);

  CGGlyph hash[] = {[self glyphForChar:'#']};

  CGContextSetRGBFillColor(context, 1.0, 0.0, 0.0, 0.5);
  CGContextShowGlyphsAtPoint(context, 30.0, 90.0, hash, 1);
  CGContextSetRGBFillColor(context, 0.0, 1.0, 0.0, 0.5);
  CGContextShowGlyphsAtPoint(context, 60.0, 90.0, hash, 1);
  CGContextSetRGBFillColor(context, 0.0, 0.0, 1.0, 0.5);
  CGContextShowGlyphsAtPoint(context, 90.0, 90.0, hash, 1);
  CGContextSetRGBFillColor(context, 0.0, 0.0, 0.0, 1.0);
  CGContextShowGlyphsAtPoint(context, 130.0, 90.0, hash, 1);

  summaryLabel.text = [NSString
      stringWithUTF8String:"test4: three a=0.5 '#' overlapping + one opaque"];
  paramsLabel1.text =
      [NSString stringWithUTF8String:
                    "passes: red(.5)@30 green(.5)@60 blue(.5)@90 black(1)@130"];
  paramsLabel2.text = [NSString
      stringWithUTF8String:"LiberationMono-Regular 48pt, baseline y=90"];

  NSLog([NSString
      stringWithUTF8String:"CGFont/CGGlyph test4: 4 CGContextShowGlyphsAtPoint "
                           "w/ alpha blending"]);

  [self presentContext:context];
}

// Test 5: CTM rotation. CGContextShowGlyphsAtPoint emits glyphs through the
// current transformation, so rotating the CTM around the bitmap centre should
// fan a "tXy" run around like the spokes of a wheel.
- (void)test5 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 16.0);

  CGGlyph glyphs[] = {[self glyphForChar:'t'], [self glyphForChar:'X'],
                      [self glyphForChar:'y']};
  size_t count = sizeof(glyphs) / sizeof(glyphs[0]);

  int steps = 8;
  for (int i = 0; i < steps; i++) {
    CGContextSaveGState(context);
    CGContextTranslateCTM(context, (CGFloat)BITMAP_WIDTH / 2.0,
                          (CGFloat)BITMAP_HEIGHT / 2.0);
    CGContextRotateCTM(context,
                       (CGFloat)i * (CGFloat)(2.0 * M_PI) / (CGFloat)steps);
    CGContextShowGlyphsAtPoint(context, 30.0, 0.0, glyphs, count);
    CGContextRestoreGState(context);
  }

  summaryLabel.text = [NSString
      stringWithUTF8String:"test5: \"tXy\" rotated around centre, 8 steps"];
  paramsLabel1.text =
      [NSString stringWithUTF8String:
                    "each step: save/translate centre/rotate/show/restore"];
  paramsLabel2.text = [self describeGlyphs:glyphs count:count];
  paramsLabel3.text = [NSString
      stringWithUTF8String:"centre=(BITMAP_WIDTH/2, BITMAP_HEIGHT/2)"];

  NSLog([NSString
      stringWithUTF8String:
          "CGFont/CGGlyph test5: 8 rotated CGContextShowGlyphsAtPoint runs"]);

  [self presentContext:context];
}

// Test 6: explicit CGContextSetTextMatrix. Hand-rolls an upright and a
// Y-mirrored text matrix to make sure CGContextShowGlyphsAtPoint honours
// both.
- (void)test6 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 1.0); // overridden by CGContextSetTextMatrix

  // CGGlyph glyphs[] = {[self glyphForChar:'L']};
  CGGlyph glyphs[] = {[self glyphForChar:'M'], [self glyphForChar:'i'],
                      [self glyphForChar:'r'], [self glyphForChar:'r'],
                      [self glyphForChar:'o'], [self glyphForChar:'r']};
  size_t count = sizeof(glyphs) / sizeof(glyphs[0]);

  CGAffineTransform upright =
      CGAffineTransformMake(32.0, 0.0, 0.0, 32.0, 0.0, 0.0);
  CGContextSetTextMatrix(context, upright);
  CGContextShowGlyphsAtPoint(context, 10.0, 100.0, glyphs, count);

  CGAffineTransform mirrored =
      CGAffineTransformMake(32.0, 0.0, 0.0, -32.0, 0.0, 0.0);
  CGContextSetTextMatrix(context, mirrored);
  CGContextShowGlyphsAtPoint(context, 10.0, 30.0, glyphs, count);

  summaryLabel.text = [NSString
      stringWithUTF8String:"test6: upright then Y-mirrored \"Mirror\""];
  paramsLabel1.text = [NSString
      stringWithUTF8String:"upright matrix: a=32 b=0 c=0 d=32 tx=0 ty=0"];
  paramsLabel2.text = [NSString
      stringWithUTF8String:"mirrored matrix: a=32 b=0 c=0 d=-32 tx=0 ty=0"];
  paramsLabel3.text = [self describeGlyphs:glyphs count:count];

  NSLog([NSString
      stringWithUTF8String:"CGFont/CGGlyph test6: CGContextSetTextMatrix "
                           "upright vs Y-mirrored"]);

  [self presentContext:context];
}

// Test 7: rotation + scale via CGContextSetTextMatrix. Bakes a growing scale
// and a step rotation into the text matrix (a=s*cos b=s*sin c=-s*sin d=s*cos),
// leaving the CTM untouched, so CGContextShowGlyphsAtPoint must honour both
// the text matrix's rotation and scale to fan "tXy" around the bitmap centre
// at increasing size.
- (void)test7 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 1.0); // overridden by CGContextSetTextMatrix

  CGGlyph glyphs[] = {[self glyphForChar:'t'], [self glyphForChar:'X'],
                      [self glyphForChar:'y']};
  size_t count = sizeof(glyphs) / sizeof(glyphs[0]);

  CGFloat cx = (CGFloat)BITMAP_WIDTH / 2.0;
  CGFloat cy = (CGFloat)BITMAP_HEIGHT / 2.0;
  CGFloat baseScale = 6.0;
  CGFloat scaleStep = 2.0;
  CGFloat radius = 30.0;
  int steps = 8;
  for (int i = 0; i < steps; i++) {
    CGFloat angle = (CGFloat)i * (CGFloat)(2.0 * M_PI) / (CGFloat)steps;
    CGFloat c = (CGFloat)cos(angle);
    CGFloat s = (CGFloat)sin(angle);
    CGFloat scale = baseScale + scaleStep * (CGFloat)i;
    CGAffineTransform rotated = CGAffineTransformMake(
        scale * c, scale * s, -scale * s, scale * c, 0.0, 0.0);
    CGContextSetTextMatrix(context, rotated);
    CGContextShowGlyphsAtPoint(context, cx + radius * c, cy + radius * s,
                               glyphs, count);
  }

  summaryLabel.text = [NSString
      stringWithUTF8String:
          "test7: \"tXy\" rotated+scaled via CGContextSetTextMatrix, 8 steps"];
  paramsLabel1.text =
      [NSString stringWithUTF8String:
                    "text matrix: a=s*cos b=s*sin c=-s*sin d=s*cos, s=6+2*i"];
  paramsLabel2.text = [self describeGlyphs:glyphs count:count];
  paramsLabel3.text = [NSString
      stringWithUTF8String:"centre=(BITMAP_WIDTH/2, BITMAP_HEIGHT/2), r=30"];

  NSLog([NSString
      stringWithUTF8String:"CGFont/CGGlyph test7: 8 CGContextSetTextMatrix "
                           "rotation+scale passes"]);

  [self presentContext:context];
}

// Test 8: squashed / distorted text via CGContextSetTextMatrix. Drives the
// text matrix with non-uniform x/y scales and a shear to confirm
// CGContextShowGlyphsAtPoint honours arbitrary 2x2 components, not just
// uniform scale + rotation. Four passes: wide, tall, sheared (italic-like),
// and combined shear+non-uniform-scale.
- (void)test8 {
  CGContextRef context = [self makeContext];
  CGContextSetFont(context, [self testFont]);
  CGContextSetFontSize(context, 1.0); // overridden by CGContextSetTextMatrix

  CGGlyph glyphs[] = {[self glyphForChar:'S'], [self glyphForChar:'q'],
                      [self glyphForChar:'u'], [self glyphForChar:'a'],
                      [self glyphForChar:'s'], [self glyphForChar:'h']};
  size_t count = sizeof(glyphs) / sizeof(glyphs[0]);

  // Wide: sx=48 sy=16, glyphs stretched horizontally.
  CGAffineTransform wide =
      CGAffineTransformMake(48.0, 0.0, 0.0, 16.0, 0.0, 0.0);
  CGContextSetTextMatrix(context, wide);
  CGContextShowGlyphsAtPoint(context, 10.0, 30.0, glyphs, count);

  // Tall: sx=12 sy=40, glyphs squashed horizontally / stretched vertically.
  CGAffineTransform tall =
      CGAffineTransformMake(12.0, 0.0, 0.0, 40.0, 0.0, 0.0);
  CGContextSetTextMatrix(context, tall);
  CGContextShowGlyphsAtPoint(context, 10.0, 70.0, glyphs, count);

  // Sheared (italic-like): sx=24 sy=24 with c=12 (x sheared by y).
  CGAffineTransform sheared =
      CGAffineTransformMake(24.0, 0.0, 12.0, 24.0, 0.0, 0.0);
  CGContextSetTextMatrix(context, sheared);
  CGContextShowGlyphsAtPoint(context, 10.0, 120.0, glyphs, count);

  // Combined: non-uniform scale plus shear in both axes.
  CGAffineTransform skewed =
      CGAffineTransformMake(32.0, 6.0, 10.0, 20.0, 0.0, 0.0);
  CGContextSetTextMatrix(context, skewed);
  CGContextShowGlyphsAtPoint(context, 80.0, 80.0, glyphs, count);

  summaryLabel.text = [NSString
      stringWithUTF8String:
          "test8: \"Squash\" wide/tall/shear/skew via CGContextSetTextMatrix"];
  paramsLabel1.text =
      [NSString stringWithUTF8String:
                    "wide a=48 d=16; tall a=12 d=40; sheared a=24 c=12 d=24"];
  paramsLabel2.text =
      [NSString stringWithUTF8String:"skewed a=32 b=6 c=10 d=20"];
  paramsLabel3.text = [self describeGlyphs:glyphs count:count];

  NSLog([NSString
      stringWithUTF8String:
          "CGFont/CGGlyph test8: 4 CGContextSetTextMatrix distortion passes"]);

  [self presentContext:context];
}

@end
