/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#ifndef TOUCHHLE_SYSTEM_HEADERS_H
#define TOUCHHLE_SYSTEM_HEADERS_H

// This file contains definitions of types etc we don't have in our SDK, which
// is built from open-source headers.

#include <CoreFoundation/CFData.h>
#include <CoreFoundation/CFDate.h>
#include <stdbool.h>
#include <stddef.h>

// Objective-C runtime

typedef signed char BOOL;
#define YES 1
#define NO 0

typedef unsigned long NSUInteger;
typedef signed long NSInteger;

#define nil ((id)0)

// id objc_msgSend(id, SEL, ...);

// Foundation

@interface NSObject {
  Class isa;
}
+ (Class)class;
+ (instancetype)alloc;
+ (instancetype)new;
- (instancetype)init;
- (instancetype)retain;
- (void)release;
- (instancetype)autorelease;
- (void)dealloc;
- (NSUInteger)retainCount;
- (id)performSelector:(SEL)selector;
- (BOOL)respondsToSelector:(SEL)selector;
@end

@interface NSAutoreleasePool : NSObject
+ (void)addObject:(id)anObject;
- (void)addObject:(id)anObject;
- (void)drain;
@end

@interface NSArray<ObjectType> : NSObject
- (NSUInteger)count;
- (ObjectType)objectAtIndex:(NSUInteger)index;
@end

@interface NSSet<ObjectType> : NSObject
- (ObjectType)anyObject;
@end

@interface NSString : NSObject
+ (instancetype)stringWithFormat:(NSString *)format, ...;
+ (instancetype)stringWithUTF8String:(const char *)string;
@end

@interface NSValue : NSObject
@end

@interface NSNumber : NSValue
+ (NSNumber *)numberWithFloat:(float)value;
+ (NSNumber *)numberWithBool:(bool)value;
@end

NSString *NSStringFromClass(Class);

typedef double NSTimeInterval;

@interface NSProcessInfo : NSObject
+ (instancetype)processInfo;
- (NSTimeInterval)systemUptime;
@end

@interface NSTimer : NSObject
+ (instancetype)timerWithTimeInterval:(NSTimeInterval)interval
                               target:(id)target
                             selector:(SEL)selector
                             userInfo:(id)user_info
                              repeats:(BOOL)repeats;
+ (instancetype)scheduledTimerWithTimeInterval:(NSTimeInterval)interval
                                        target:(id)target
                                      selector:(SEL)selector
                                      userInfo:(id)user_info
                                       repeats:(BOOL)repeats;
- (void)invalidate;
@end

SEL NSSelectorFromString(NSString *);

// Core Graphics

// (See CGAffineTransform.c for where this define comes from.)
#ifdef DEFINE_ME_WHEN_BUILDING_ON_MACOS
typedef double CGFloat; // 64-bit definition (not supported by touchHLE)
#else
typedef float CGFloat;
#endif

typedef struct {
  CGFloat x, y;
} CGPoint;
bool CGPointEqualToPoint(CGPoint, CGPoint);
static inline CGPoint CGPointMake(CGFloat x, CGFloat y) {
  return (CGPoint){x, y};
}
typedef struct {
  CGFloat width, height;
} CGSize;
bool CGSizeEqualToSize(CGSize, CGSize);
static inline CGSize CGSizeMake(CGFloat width, CGFloat height) {
  return (CGSize){width, height};
}
typedef struct {
  CGPoint origin;
  CGSize size;
} CGRect;
bool CGRectEqualToRect(CGRect, CGRect);
static inline CGRect CGRectMake(CGFloat x, CGFloat y, CGFloat width,
                                CGFloat height) {
  return (CGRect){CGPointMake(x, y), CGSizeMake(width, height)};
}

typedef struct {
  CGFloat a, b, c, d, tx, ty;
} CGAffineTransform;
extern const CGAffineTransform CGAffineTransformIdentity;
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
CGAffineTransform CGAffineTransformInvert(CGAffineTransform);
CGPoint CGPointApplyAffineTransform(CGPoint, CGAffineTransform);
CGSize CGSizeApplyAffineTransform(CGSize, CGAffineTransform);
CGRect CGRectApplyAffineTransform(CGRect, CGAffineTransform);

@interface NSValue (CGGeometryNSValueAdditions)
+ (instancetype)valueWithCGPoint:(CGPoint)point;
+ (instancetype)valueWithCGRect:(CGRect)rect;
@end

// `CGDataProvider.h`

typedef struct _CGDataProvider *CGDataProviderRef;

CGDataProviderRef CGDataProviderCreateWithCFData(CFDataRef);
CFDataRef CGDataProviderCopyData(CGDataProviderRef);

// `CGGeometry.h`

CGFloat CGRectGetMinX(CGRect);
CGFloat CGRectGetMaxX(CGRect);
CGFloat CGRectGetMinY(CGRect);
CGFloat CGRectGetMaxY(CGRect);
CGFloat CGRectGetHeight(CGRect);
CGFloat CGRectGetWidth(CGRect);

// `CGImage.h`

typedef struct _CGImage *CGImageRef;

CGImageRef CGImageCreateWithJPEGDataProvider(CGDataProviderRef, const CGFloat *,
                                             bool, int);
size_t CGImageGetWidth(CGImageRef);
size_t CGImageGetHeight(CGImageRef);
CGDataProviderRef CGImageGetDataProvider(CGImageRef);

// `CGColor.h`

typedef struct _CGColor *CGColorRef;

CGColorRef CGColorCreateGenericRGB(CGFloat red, CGFloat green, CGFloat blue,
                                   CGFloat alpha);

// Core Animation
typedef NSString *CAMediaTimingFunctionName;

CFTimeInterval CACurrentMediaTime();

@interface CAMediaTimingFunction : NSObject
+ (instancetype)functionWithName:(CAMediaTimingFunctionName)name;
@end
@interface CAAnimation : NSObject
- (void)setTimingFunction:(CAMediaTimingFunction *)timingFunction;
- (CFTimeInterval)duration;
- (void)setDuration:(CFTimeInterval)duration;
- (void)setBeginTime:(CFTimeInterval)beginTime;
- (void)setRepeatCount:(float)repeatCount;
- (void)setAutoreverses:(bool)autoreverses;
@end
@interface CAPropertyAnimation : CAAnimation
+ (instancetype)animationWithKeyPath:(NSString *)path;
@end
@interface CABasicAnimation : CAPropertyAnimation
- (void)setFromValue:(id)value;
- (void)setToValue:(id)value;
@end
@interface CALayer : NSObject
- (void)setAffineTransform:(CGAffineTransform)transform;
- (void)setAnchorPoint:(CGPoint)point;
- (void)setCornerRadius:(CGFloat)radius;
- (CGPoint)position;
- (void)setPosition:(CGPoint)position;
- (CGRect)bounds;
- (void)setBounds:(CGRect)bounds;
- (CGPoint)convertPoint:(CGPoint)point fromLayer:(CALayer *)layer;
- (CGPoint)convertPoint:(CGPoint)point toLayer:(CALayer *)layer;
- (CGRect)convertRect:(CGRect)point fromLayer:(CALayer *)layer;
- (CGRect)convertRect:(CGRect)point toLayer:(CALayer *)layer;
- (void)addAnimation:(CAAnimation *)anim forKey:(NSString *)key;
- (void)removeAnimationForKey:(NSString *)key;
@end

// UIKit

typedef enum {
  UITextAlignmentLeft = 0,
  UITextAlignmentCenter = 1,
  UITextAlignmentRight = 2,
} UITextAlignment;

typedef enum {
  UIButtonTypeRoundedRect = 1,
} UIButtonType;

typedef enum {
  UIControlStateNormal = 0,
} UIControlState;

typedef enum {
  UIControlEventTouchUpInside = 1 << 6,
} UIControlEvents;

@interface UIApplication : NSObject
+ (instancetype)sharedApplication;
- (id)delegate;
@end
@interface UIScreen : NSObject
+ (instancetype)mainScreen;
- (CGRect)applicationFrame;
@end
@interface UIColor : NSObject
+ (instancetype)colorWithRed:(CGFloat)r
                       green:(CGFloat)g
                        blue:(CGFloat)b
                       alpha:(CGFloat)a;
+ (instancetype)colorWithWhite:(CGFloat)w alpha:(CGFloat)a;
+ (instancetype)clearColor;
+ (instancetype)blackColor;
+ (instancetype)whiteColor;
+ (instancetype)darkGrayColor;
+ (instancetype)grayColor;
+ (instancetype)lightGrayColor;
+ (instancetype)blueColor;
+ (instancetype)brownColor;
+ (instancetype)cyanColor;
+ (instancetype)greenColor;
+ (instancetype)magentaColor;
+ (instancetype)orangeColor;
+ (instancetype)purpleColor;
+ (instancetype)redColor;
+ (instancetype)yellowColor;
@end
@interface UIEvent : NSObject
@end
@class UIView;
@interface UITouch : NSObject
- (CGPoint)locationInView:(UIView *)view;
@end
@interface UIResponder : NSObject
@end
@class UIWindow;
@interface UIView : UIResponder
- (instancetype)initWithFrame:(CGRect)frame;
- (CALayer *)layer;
- (CGRect)bounds;
- (CGRect)frame;
- (void)setBounds:(CGRect)bounds;
- (void)setFrame:(CGRect)frame;
- (void)setTransform:(CGAffineTransform)transform;
- (CGPoint)convertPoint:(CGPoint)point fromView:(UIView *)view;
- (CGPoint)convertPoint:(CGPoint)point toView:(UIView *)view;
- (CGRect)convertRect:(CGRect)point fromView:(UIView *)view;
- (CGRect)convertRect:(CGRect)point toView:(UIView *)view;
- (UIView *)hitTest:(CGPoint)point withEvent:(UIEvent *)event;
- (UIWindow *)window;
- (void)addSubview:(UIView *)view;
- (void)removeFromSuperview;
- (NSArray<UIView *> *)subviews;
- (void)layoutSubviews;
- (void)setBackgroundColor:(UIColor *)color;
- (CGFloat)alpha;
- (void)setAlpha:(CGFloat)alpha;
- (BOOL)isHidden;
- (void)setHidden:(BOOL)hidden;
- (CALayer *)layer;
@end
@interface UIWindow : UIView
- (void)makeKeyAndVisible;
- (CGPoint)convertPoint:(CGPoint)point fromWindow:(UIWindow *)window;
- (CGPoint)convertPoint:(CGPoint)point toWindow:(UIWindow *)window;
- (CGRect)convertRect:(CGRect)point fromWindow:(UIWindow *)window;
- (CGRect)convertRect:(CGRect)point toWindow:(UIWindow *)window;
@end
@interface UILabel : UIView
- (void)setText:(NSString *)text;
- (void)setTextAlignment:(UITextAlignment)alignment;
- (void)setTextColor:(UIColor *)color;
@end
@interface UIControl : UIView
- (void)addTarget:(id)target
              action:(SEL)action
    forControlEvents:(UIControlEvents)events;
@end
@interface UIButton : UIControl
+ (instancetype)buttonWithType:(UIButtonType)type;
- (void)setTitle:(NSString *)title forState:(UIControlState)state;
@end

int UIApplicationMain(int, char **, NSString *, NSString *);

NSString *NSStringFromCGPoint(CGPoint);
NSString *NSStringFromCGSize(CGSize);
NSString *NSStringFromCGRect(CGRect);

#endif // TOUCHHLE_SYSTEM_HEADERS_H
