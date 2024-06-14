/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIColor`.

use super::ui_graphics::UIGraphicsGetCurrentContext;
use crate::frameworks::core_graphics::cg_color::{CGColorRef, CGColorRelease, CGColorRetain};
use crate::frameworks::core_graphics::cg_context::CGContextSetRGBFillColor;
use crate::frameworks::core_graphics::{cg_color, CGFloat};
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr, ObjC,
    SEL,
};
use crate::Environment;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    standard_colors: HashMap<SEL, id>,
}

fn get_standard_color(
    env: &mut Environment,
    sel: SEL,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) -> id {
    if let Some(&existing) = env.framework_state.uikit.ui_color.standard_colors.get(&sel) {
        existing
    } else {
        let new: id = msg_class![env; _touchHLE_UIColor_Static alloc];
        let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
        env.framework_state
            .uikit
            .ui_color
            .standard_colors
            .insert(sel, new);
        new
    }
}

struct UIColorHostObject {
    cg_color: CGColorRef,
}
impl HostObject for UIColorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIColor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIColorHostObject {
        cg_color: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)colorWithCGColor:(CGColorRef)cg_color {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCGColor:cg_color];
    autorelease(env, new)
}

+ (id)colorWithRed:(CGFloat)r
             green:(CGFloat)g
              blue:(CGFloat)b
             alpha:(CGFloat)a {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
    autorelease(env, new)
}

+ (id)colorWithWhite:(CGFloat)w alpha:(CGFloat)a {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithWhite:w alpha:a];
    autorelease(env, new)
}

+ (id)clearColor    { get_standard_color(env, _cmd, 0.0, 0.0, 0.0, 0.0) }
+ (id)blackColor    { get_standard_color(env, _cmd, 0.0, 0.0, 0.0, 1.0) }
+ (id)whiteColor    { get_standard_color(env, _cmd, 1.0, 1.0, 1.0, 1.0) }
+ (id)darkGrayColor {
    get_standard_color(env, _cmd, 1.0/3.0, 1.0/3.0, 1.0/3.0, 1.0)
}
+ (id)grayColor {
    get_standard_color(env, _cmd, 1.0/2.0, 1.0/2.0, 1.0/2.0, 1.0)
}
+ (id)lightGrayColor {
    get_standard_color(env, _cmd, 2.0/3.0, 2.0/3.0, 2.0/3.0, 1.0)
}
+ (id)blueColor     { get_standard_color(env, _cmd, 0.0, 0.0, 1.0, 1.0) }
+ (id)brownColor    { get_standard_color(env, _cmd, 0.6, 0.4, 0.2, 1.0) }
+ (id)cyanColor     { get_standard_color(env, _cmd, 0.0, 1.0, 1.0, 1.0) }
+ (id)greenColor    { get_standard_color(env, _cmd, 0.0, 1.0, 0.0, 1.0) }
+ (id)magentaColor  { get_standard_color(env, _cmd, 1.0, 0.0, 1.0, 1.0) }
+ (id)orangeColor   { get_standard_color(env, _cmd, 1.0, 0.5, 0.0, 1.0) }
+ (id)purpleColor   { get_standard_color(env, _cmd, 0.5, 0.0, 1.5, 1.0) }
+ (id)redColor      { get_standard_color(env, _cmd, 1.0, 0.0, 0.0, 1.0) }
+ (id)yellowColor   { get_standard_color(env, _cmd, 1.0, 1.0, 0.0, 1.0) }

// TODO: more initializers, set methods, more accessors

- (id)initWithCGColor:(CGColorRef)cg_color {
    CGColorRetain(env, cg_color);
    env.objc.borrow_mut::<UIColorHostObject>(this).cg_color = cg_color;
    this
}

- (id)initWithWhite:(CGFloat)w alpha:(CGFloat)a {
    let w = w.clamp(0.0, 1.0);
    let a = a.clamp(0.0, 1.0);

    env.objc.borrow_mut::<UIColorHostObject>(this).cg_color = cg_color::from_rgba(env, (w, w, w, a));

    this
}

- (id)initWithRed:(CGFloat)r
            green:(CGFloat)g
             blue:(CGFloat)b
            alpha:(CGFloat)a {
    env.objc.borrow_mut::<UIColorHostObject>(this).cg_color = cg_color::from_rgba(env, (r, g, b, a));
    this
}

- (bool)getRed:(MutPtr<CGFloat>)r
         green:(MutPtr<CGFloat>)g
          blue:(MutPtr<CGFloat>)b
         alpha:(MutPtr<CGFloat>)a {
    let color = env.objc.borrow::<UIColorHostObject>(this).cg_color;
    let (r_, g_, b_, a_) = cg_color::to_rgba(&env.objc, color);
    env.mem.write(r, r_);
    env.mem.write(g, g_);
    env.mem.write(b, b_);
    env.mem.write(a, a_);
    true
}

- (())set {
    msg![env; this setFill]
    // TODO: set stroke color as well
}

- (())setFill {
    let context = UIGraphicsGetCurrentContext(env);
    assert_ne!(context, nil);
    let (r, g, b, a) = get_rgba(&env.objc, this);
    CGContextSetRGBFillColor(env, context, r, g, b, a);
}

- (CGColorRef)CGColor {
    env.objc.borrow::<UIColorHostObject>(this).cg_color
}

- (())dealloc {
    let color = env.objc.borrow_mut::<UIColorHostObject>(this).cg_color;
    CGColorRelease(env, color);

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// Special subclass for standard colors with a static lifetime.
// See `get_standard_color`.
@implementation _touchHLE_UIColor_Static: UIColor

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIColorHostObject {
        cg_color: nil,
    });
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id) retain { this }
- (()) release {}
- (id) autorelease { this }

@end

};

/// Shortcut for use in Core Animation's compositor: get the RGBA triple for a
/// `UIColor*`.
pub fn get_rgba(objc: &ObjC, ui_color: id) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    let color = objc.borrow::<UIColorHostObject>(ui_color).cg_color;
    cg_color::to_rgba(objc, color)
}
