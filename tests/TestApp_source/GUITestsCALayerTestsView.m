/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"
#include <math.h>

#include "GUITestsCALayerTestsView.h"

#define NUM_TESTS 18

@implementation GUITestsCALayerTestsView : UIView

UILabel *title;
UIView *testArea;
NSUInteger testNum;
UIView *lastTappedView;
UILabel *lastTappedLocalFrameLabel;
UILabel *lastTappedGlobalFrameLabel;

- (instancetype)initWithFrame:(CGRect)frame {
  [super initWithFrame:frame];

  title = [[UILabel alloc] initWithFrame:[self bounds]];
  title.text = [NSString stringWithUTF8String:"CALayer tests (press →)"];
  title.textAlignment = UITextAlignmentCenter;
  title.frame = CGRectMake(0, 0, 320, 20);
  [self addSubview:title];

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

  lastTappedLocalFrameLabel =
      [[UILabel alloc] initWithFrame:CGRectMake(10, 340, 300, 20)];
  lastTappedLocalFrameLabel.textColor = [UIColor whiteColor];
  lastTappedLocalFrameLabel.backgroundColor = [UIColor clearColor];
  [self addSubview:lastTappedLocalFrameLabel];
  lastTappedGlobalFrameLabel =
      [[UILabel alloc] initWithFrame:CGRectMake(10, 360, 300, 20)];
  lastTappedGlobalFrameLabel.textColor = [UIColor whiteColor];
  lastTappedGlobalFrameLabel.backgroundColor = [UIColor clearColor];
  [self addSubview:lastTappedGlobalFrameLabel];

  // Don't display any test initially. The testing for convertPoint:toLayer: etc
  // won't produce the right results until this view has actually been added to
  // the window.
  testNum = 0;

  return self;
}

- (void)dealloc {
  [title release];
  [lastTappedLocalFrameLabel release];
  [lastTappedGlobalFrameLabel release];
  [testArea release];
  [super dealloc];
}

- (void)prevTest {
  if (testNum > 1)
    testNum--;
  [self displayTest];
}
- (void)nextTest {
  if (testNum < NUM_TESTS)
    testNum++;
  [self displayTest];
}
- (void)displayTest {
  title.text = [NSString
      stringWithFormat:[NSString stringWithUTF8String:"CALayer test %u/%u"],
                       testNum, NUM_TESTS];
  [testArea removeFromSuperview];
  [testArea release];
  lastTappedView = nil;
  lastTappedLocalFrameLabel.text = [NSString stringWithUTF8String:""];
  lastTappedGlobalFrameLabel.text = [NSString stringWithUTF8String:""];
  testArea = [[UIView alloc] initWithFrame:CGRectMake(10, 30, 300, 300)];
  testArea.backgroundColor = [UIColor grayColor];
  [self addSubview:testArea];

  [self performSelector:NSSelectorFromString([NSString
                            stringWithFormat:[NSString
                                                 stringWithUTF8String:"test%u"],
                                             testNum])];
}

- (void)tick {
  if (lastTappedView) {
    lastTappedLocalFrameLabel.text = NSStringFromCGRect(lastTappedView.frame);
    lastTappedGlobalFrameLabel.text =
        NSStringFromCGRect([testArea convertRect:lastTappedView.frame
                                        fromView:lastTappedView]);
  }
  SEL tickSelector = NSSelectorFromString([NSString
      stringWithFormat:[NSString stringWithUTF8String:"test%uTick"], testNum]);
  if ([self respondsToSelector:tickSelector]) {
    [self performSelector:tickSelector];
  }
}

- (UIView *)addViewWithFrame:(CGRect)frame color:(UIColor *)color {
  return [self addViewWithFrame:frame color:color superview:testArea];
}
- (UIView *)addViewWithFrame:(CGRect)frame
                       color:(UIColor *)color
                   superview:(UIView *)parent {
  UIView *view = [[UIView alloc] initWithFrame:frame];
  view.backgroundColor = color;
  [parent addSubview:view];
  [view release];
  return view;
}
- (UILabel *)addLabelWithFrame:(CGRect)frame text:(NSString *)text {
  UILabel *label = [[UILabel alloc] initWithFrame:frame];
  label.text = text;
  label.textColor = [UIColor whiteColor];
  label.backgroundColor = [UIColor clearColor];
  [testArea addSubview:label];
  [label release];
  return label;
}

- (void)touchesBegan:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  CGPoint locationInView = [[touches anyObject] locationInView:testArea];
  UIView *hitView = [testArea hitTest:locationInView withEvent:event];
  hitView.alpha = 1.0 - (hitView.alpha - 0.5);
  lastTappedView = hitView;
}

// These tests should all look like three squares arranged diagonally (in some
// cases with animation).
// The color differences make it more obvious when you've switched tests.

- (void)test1 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor redColor]];
  [self addLabelWithFrame:CGRectMake(0, 0, 100, 25)
                     text:NSStringFromCGPoint([view1.layer
                              convertPoint:CGPointMake(0.0, 0.0)
                                 fromLayer:nil])];
  [self addLabelWithFrame:CGRectMake(0, 25, 100, 25)
                     text:NSStringFromCGPoint([view1
                              convertPoint:CGPointMake(0.0, 0.0)
                                  fromView:nil])];
  [self addLabelWithFrame:CGRectMake(0, 50, 100, 25)
                     text:NSStringFromCGPoint([view1.layer
                              convertPoint:CGPointMake(0.0, 0.0)
                                   toLayer:view1.window.layer])];
  [self addLabelWithFrame:CGRectMake(0, 75, 100, 25)
                     text:NSStringFromCGPoint([view1.window
                              convertPoint:CGPointMake(0.0, 0.0)
                                  toWindow:nil])];
  [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                   color:[UIColor greenColor]];
  [self addViewWithFrame:CGRectMake(200, 200, 100, 100)
                   color:[UIColor blueColor]];
}
- (void)test2 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor cyanColor]];
  view1.layer.anchorPoint = CGPointMake(0.0, 0.0);
  view1.layer.position = CGPointMake(0.0, 0.0);
  UIView *view2 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor magentaColor]];
  view2.layer.anchorPoint = CGPointMake(0.5, 0.5);
  view2.layer.position = CGPointMake(150.0, 150.0);
  UIView *view3 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor yellowColor]];
  view3.layer.anchorPoint = CGPointMake(1.0, 1.0);
  view3.layer.position = CGPointMake(300.0, 300.0);
}
- (void)test3 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor orangeColor]];
  view1.layer.affineTransform = CGAffineTransformMakeTranslation(0.0, 0.0);
  UIView *view2 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor greenColor]];
  view2.layer.affineTransform = CGAffineTransformMakeTranslation(100.0, 100.0);
  UIView *view3 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor purpleColor]];
  view3.layer.anchorPoint = CGPointMake(1.0, 1.0);
  view3.layer.position = CGPointMake(200.0, 200.0);
  view3.layer.affineTransform = CGAffineTransformMakeTranslation(100.0, 100.0);
}
- (void)test4 {
  [self test2];
}
- (void)test4Tick {
  NSTimeInterval t = [[NSProcessInfo processInfo] systemUptime];
  t -= floor(t);
  for (NSUInteger i = 0; i < testArea.subviews.count; i++) {
    [testArea.subviews objectAtIndex:i].transform =
        CGAffineTransformMakeRotation(t * M_PI * 2);
  }
}
- (void)test5 {
  [self test2];
}
- (void)test5Tick {
  NSTimeInterval t = [[NSProcessInfo processInfo] systemUptime];
  t -= floor(t);
  for (NSUInteger i = 0; i < testArea.subviews.count; i++) {
    ((UIView *)[testArea.subviews objectAtIndex:i]).transform =
        CGAffineTransformMakeScale(0.5 + (sin(t * M_PI * 2) / 4 + 0.25),
                                   0.5 + (cos(t * M_PI * 2) / 4 + 0.25));
  }
}
- (void)test6 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor redColor]];
  UIView *view2 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor greenColor]
                               superview:view1];
  view2.transform = CGAffineTransformMakeTranslation(100.0, 100.0);
  UIView *view3 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor blueColor]
                               superview:view2];
  view3.transform = CGAffineTransformMakeTranslation(100.0, 100.0);
}

- (void)test7 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor cyanColor]];
  view1.bounds = CGRectMake(100.0, 100.0, 100.0, 100.0);
  UIView *view2 = [self addViewWithFrame:CGRectMake(200, 200, 100, 100)
                                   color:[UIColor magentaColor]
                               superview:view1];
  UIView *view3 = [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                                   color:[UIColor yellowColor]
                               superview:view2];
}

// These tests also use three squares, but they break the earlier rule that the
// squares should always have the same arrangement; the transforms will shift
// their positions.

- (void)test8 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor orangeColor]];
  UIView *view2 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor greenColor]
                               superview:view1];
  view2.transform = CGAffineTransformRotate(
      CGAffineTransformMakeTranslation(100.0, 100.0), M_PI / 8);
  UIView *view3 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor purpleColor]
                               superview:view2];
  view3.transform = CGAffineTransformRotate(
      CGAffineTransformMakeTranslation(100.0, 100.0), M_PI / 8);
}
- (void)test9 {
  UIView *view1 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor cyanColor]];
  UIView *view2 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor magentaColor]
                               superview:view1];
  view2.transform = CGAffineTransformScale(
      CGAffineTransformMakeTranslation(100.0, 100.0), 0.75, 0.75);
  UIView *view3 = [self addViewWithFrame:CGRectMake(0, 0, 100, 100)
                                   color:[UIColor yellowColor]
                               superview:view2];
  view3.transform = CGAffineTransformScale(
      CGAffineTransformMakeTranslation(100.0, 100.0), 0.75, 0.75);
}

// These tests don't have the three squares pattern.
- (void)test10 {
  [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                   color:[UIColor blackColor]];
  [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                   color:[UIColor redColor]];
}
- (void)test10Tick {
  NSTimeInterval t = [[NSProcessInfo processInfo] systemUptime];
  t -= floor(t);
  [testArea.subviews objectAtIndex:1].transform =
      CGAffineTransformMakeRotation(t * M_PI * 2);
  [testArea.subviews objectAtIndex:0].frame =
      [testArea.subviews objectAtIndex:1].frame;
}
- (void)test11 {
  [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                   color:[UIColor redColor]];
  UIView *view2 = [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                                   color:[UIColor blackColor]];
  view2.transform = CGAffineTransformMakeRotation(M_PI / 4);
}
- (void)test11Tick {
  NSTimeInterval t = [[NSProcessInfo processInfo] systemUptime];
  t -= floor(t);
  [testArea.subviews objectAtIndex:0].transform = CGAffineTransformMakeScale(
      1.25 + sin(t * M_PI * 2) / 4, 1.25 + sin(t * M_PI * 2) / 4);
  [testArea.subviews objectAtIndex:1].frame =
      [testArea.subviews objectAtIndex:0].frame;
}
- (void)test12 {
  [self addViewWithFrame:CGRectMake(100, 100, 100, 100)
                   color:[UIColor redColor]];
  UIView *view2 = [self addViewWithFrame:CGRectMake(50, 50, 200, 200)
                                   color:[UIColor clearColor]];
  [self addViewWithFrame:CGRectMake(0, 0, 50, 50)
                   color:[UIColor blackColor]
               superview:view2];
  [self addViewWithFrame:CGRectMake(150, 150, 50, 50)
                   color:[UIColor blackColor]
               superview:view2];
}
- (void)test12Tick {
  NSTimeInterval t = [[NSProcessInfo processInfo] systemUptime];
  [testArea.subviews objectAtIndex:1].transform = CGAffineTransformRotate(
      CGAffineTransformMakeScale(0.5, 0.5), (M_PI / 2) * (((int)floor(t)) % 4));
  [self test11Tick];
}

// CAAnimation tests
- (void)test13 {
  UILabel* label = [self addLabelWithFrame:CGRectMake(75, 75, 175, 175) text:[NSString stringWithUTF8String:"hello, animations!"]];

  CALayer *layer = [label layer];
  [layer setBackgroundColor:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [layer setCornerRadius:32.0];

  CAMediaTimingFunction *easeInEaseOut = [CAMediaTimingFunction
      functionWithName:[NSString stringWithUTF8String:"easeInEaseOut"]];

  CABasicAnimation *animation = [CABasicAnimation
      animationWithKeyPath:[NSString stringWithUTF8String:"opacity"]];
  [animation setTimingFunction:easeInEaseOut];
  [animation setDuration:3.0];
  [animation setFromValue:[NSNumber numberWithFloat:0.0]];
  [animation setToValue:[NSNumber numberWithFloat:1.0]];
  [layer addAnimation:animation
               forKey:[NSString stringWithUTF8String:"opacity_animation"]];
}
- (void)test14 {
  UILabel* label = [self addLabelWithFrame:CGRectMake(75, 75, 175, 175) text:[NSString stringWithUTF8String:"hello, animations!"]];

  CALayer *layer = [label layer];
  [layer setBackgroundColor:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [layer setCornerRadius:32.0];

  CABasicAnimation *animation = [CABasicAnimation
      animationWithKeyPath:[NSString stringWithUTF8String:"backgroundColor"]];
  [animation setDuration:4.0];
  [animation setRepeatCount:3.5];
  [animation setAutoreverses:true];
  [animation setFromValue:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [animation setToValue:CGColorCreateGenericRGB(0.75, 0.00, 0.25, 1.0)];
  [layer addAnimation:animation
               forKey:[NSString stringWithUTF8String:"bg_animation"]];
}
- (void)test15 {
  UILabel* label = [self addLabelWithFrame:CGRectMake(75, 75, 175, 175) text:[NSString stringWithUTF8String:"hello, animations!"]];

  CALayer *layer = [label layer];
  [layer setBackgroundColor:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [layer setCornerRadius:32.0];

  CAMediaTimingFunction *linear = [CAMediaTimingFunction
      functionWithName:[NSString stringWithUTF8String:"linear"]];

  CABasicAnimation *animation = [CABasicAnimation
      animationWithKeyPath:[NSString stringWithUTF8String:"hidden"]];
  [animation setTimingFunction:linear];
  [animation setDuration:4.0];
  [animation setFromValue:[NSNumber numberWithBool:false]];
  [animation setToValue:[NSNumber numberWithBool:true]];
  [layer addAnimation:animation
               forKey:[NSString stringWithUTF8String:"hidden"]];
}
- (void)test16 {
  UILabel* label = [self addLabelWithFrame:CGRectMake(75, 75, 175, 175) text:[NSString stringWithUTF8String:"hello, animations!"]];

  CALayer *layer = [label layer];
  [layer setBackgroundColor:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [layer setCornerRadius:32.0];

  CAMediaTimingFunction *easeInEaseOut = [CAMediaTimingFunction
      functionWithName:[NSString stringWithUTF8String:"easeInEaseOut"]];

  CABasicAnimation *animation = [CABasicAnimation
      animationWithKeyPath:[NSString stringWithUTF8String:"bounds"]];
  [animation setTimingFunction:easeInEaseOut];
  [animation setDuration:6.0];
  [animation setFromValue:[NSValue valueWithCGRect:CGRectMake(10, 10, 50, 10)]];
  [animation setToValue:[NSValue valueWithCGRect:[layer bounds]]];
  [layer addAnimation:animation
               forKey:[NSString stringWithUTF8String:"bounds_animation"]];
}
- (void)test17 {
  UILabel* label = [self addLabelWithFrame:CGRectMake(75, 75, 175, 175) text:[NSString stringWithUTF8String:"hello, animations!"]];

  CALayer *layer = [label layer];
  [layer setBackgroundColor:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [layer setCornerRadius:32.0];

  CABasicAnimation *animation = [CABasicAnimation
      animationWithKeyPath:[NSString stringWithUTF8String:"cornerRadius"]];
  [animation setDuration:6.0];
  [animation setFromValue:[NSNumber numberWithFloat:0.0]];
  [animation setToValue:[NSNumber numberWithFloat:32.0]];
  [layer addAnimation:animation
               forKey:[NSString stringWithUTF8String:"corner_animation"]];
}
- (void)test18 {
  UILabel* label = [self addLabelWithFrame:CGRectMake(75, 75, 175, 175) text:[NSString stringWithUTF8String:"hello, animations!"]];

  CALayer *layer = [label layer];
  [layer setBackgroundColor:CGColorCreateGenericRGB(0.0, 0.70, 0.0, 1.0)];
  [layer setCornerRadius:32.0];

  CAMediaTimingFunction *easeInEaseOut = [CAMediaTimingFunction
      functionWithName:[NSString stringWithUTF8String:"easeInEaseOut"]];

  CABasicAnimation *animation = [CABasicAnimation
      animationWithKeyPath:[NSString stringWithUTF8String:"position"]];
  [animation setTimingFunction:easeInEaseOut];
  [animation setBeginTime:CACurrentMediaTime() + 2.0];
  [animation setDuration:8.0];
  [animation setFromValue:[NSValue valueWithCGPoint:CGPointMake(120, -75)]];
  [animation setToValue:[NSValue valueWithCGPoint:[layer position]]];
  [layer addAnimation:animation
               forKey:[NSString stringWithUTF8String:"position_animation"]];

  // Remove animation that doesn't exist
  [layer
      removeAnimationForKey:[NSString
                                stringWithUTF8String:"non_existent_animation"]];
}
@end
