/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#include "system_headers.h"

#include "GUITestsAppDelegate.h"
#include "GUITestsBlendModeView.h"
#include "GUITestsCALayerTestsView.h"
#include "GUITestsCGFontGlyphTestsView.h"
#include "GUITestsMainMenu.h"

@implementation GUITestsMainMenu : UIView

UIView *ball;
CGFloat ballXVelocity;
CGFloat ballYVelocity;
UIWindow *window2;

- (instancetype)initWithFrame:(CGRect)frame {
  [super initWithFrame:frame];

  self.backgroundColor = [UIColor whiteColor];

  CGRect bounds = [self bounds];
  CGRect labelBounds = CGRectMake(bounds.origin.x, bounds.origin.y,
                                  bounds.size.width, bounds.size.height / 4.0);
  UILabel *label = [[[UILabel alloc] initWithFrame:labelBounds] autorelease];
  label.text = [NSString stringWithUTF8String:"hello, world! 🌏"];
  label.textAlignment = UITextAlignmentCenter;
  [self addSubview:label];
  ball = [[UIView alloc] initWithFrame:CGRectMake(0, 0, 20, 20)];
  ball.layer.cornerRadius = ball.frame.size.width / 2;
  ball.backgroundColor = [UIColor redColor];
  [self addSubview:ball];
  ballXVelocity = 5;
  ballYVelocity = 5;

  UIButton *button1 = [UIButton buttonWithType:UIButtonTypeRoundedRect];
  [button1 setTitle:[NSString stringWithUTF8String:"Window 2"]
           forState:UIControlStateNormal];
  [button1 setFrame:CGRectMake(40, 140, 240, 40)];
  [button1 addTarget:self
                action:@selector(toggleWindow)
      forControlEvents:UIControlEventTouchUpInside];
  [self addSubview:button1];

  UIButton *button2 = [UIButton buttonWithType:UIButtonTypeRoundedRect];
  [button2 setTitle:[NSString stringWithUTF8String:"CALayer tests"]
           forState:UIControlStateNormal];
  [button2 setFrame:CGRectMake(40, 220, 240, 40)];
  [button2 addTarget:self
                action:@selector(goToCALayerTests)
      forControlEvents:UIControlEventTouchUpInside];
  [self addSubview:button2];

  UIButton *button3 = [UIButton buttonWithType:UIButtonTypeRoundedRect];
  [button3 setTitle:[NSString stringWithUTF8String:"CGFont/CGGlyph tests"]
           forState:UIControlStateNormal];
  [button3 setFrame:CGRectMake(40, 300, 240, 40)];
  [button3 addTarget:self
                action:@selector(goToCGFontGlyphTests)
      forControlEvents:UIControlEventTouchUpInside];
  [self addSubview:button3];

  UIButton *button4 = [UIButton buttonWithType:UIButtonTypeRoundedRect];
  [button4 setTitle:[NSString stringWithUTF8String:"BlendMode tests"]
           forState:UIControlStateNormal];
  [button4 setFrame:CGRectMake(40, 380, 240, 40)];
  [button4 addTarget:self
                action:@selector(goToBlendModeTests)
      forControlEvents:UIControlEventTouchUpInside];
  [self addSubview:button4];

  return self;
}

- (void)dealloc {
  [ball release];
  [window2 release];
  [super dealloc];
}

- (void)tick {
  CGRect windowFrame = [self bounds];
  CGRect ballFrame = [ball frame];
  ballFrame.origin.x += ballXVelocity;
  ballFrame.origin.y += ballYVelocity;
  CGFloat oldXVelocity = ballXVelocity;
  CGFloat oldYVelocity = ballYVelocity;
  if (CGRectGetMaxX(ballFrame) >= CGRectGetMaxX(windowFrame)) {
    ballXVelocity = -ballXVelocity;
    ballFrame.origin.x = CGRectGetMaxX(windowFrame) - ballFrame.size.width;
  } else if (CGRectGetMinX(ballFrame) <= CGRectGetMinX(windowFrame)) {
    ballXVelocity = -ballXVelocity;
    ballFrame.origin.x = CGRectGetMinX(windowFrame);
  }
  if (CGRectGetMaxY(ballFrame) >= CGRectGetMaxY(windowFrame)) {
    ballYVelocity = -ballYVelocity;
    ballFrame.origin.y = CGRectGetMaxY(windowFrame) - ballFrame.size.height;
  } else if (CGRectGetMinY(ballFrame) <= CGRectGetMinY(windowFrame)) {
    ballYVelocity = -ballYVelocity;
    ballFrame.origin.y = CGRectGetMinY(windowFrame);
  }
  if (oldXVelocity != ballXVelocity || oldYVelocity != ballYVelocity)
    ball.backgroundColor =
        [UIColor colorWithRed:(ballFrame.origin.x / windowFrame.size.width)
                        green:((ballXVelocity + ballYVelocity) / 10.0 + 0.5)
                         blue:(ballFrame.origin.y / windowFrame.size.height)
                        alpha:1.0];
  ball.frame = ballFrame;
}

- (void)goToCALayerTests {
  [((GUITestsAppDelegate *)[[UIApplication sharedApplication]
      delegate]) setMainView:[[[GUITestsCALayerTestsView alloc]
                                 initWithFrame:[self frame]] autorelease]];
}

- (void)goToCGFontGlyphTests {
  [((GUITestsAppDelegate *)[[UIApplication sharedApplication]
      delegate]) setMainView:[[[GUITestsCGFontGlyphTestsView alloc]
                                 initWithFrame:[self frame]] autorelease]];
}

- (void)goToBlendModeTests {
  [((GUITestsAppDelegate *)[[UIApplication sharedApplication]
      delegate]) setMainView:[[[GUITestsBlendModeView alloc]
                                 initWithFrame:[self frame]] autorelease]];
}

- (void)toggleWindow {
  if (!window2) {
    window2 = [[UIWindow alloc] initWithFrame:CGRectMake(80, 0, 80, 80)];
    window2.backgroundColor = [UIColor magentaColor];
  }
  [window2 setHidden:![window2 isHidden]];
}

@end
