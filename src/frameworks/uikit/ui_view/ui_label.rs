/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UILabel`.

use crate::frameworks::core_graphics::cg_context::CGContextSetRGBFillColor;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_color;
use crate::frameworks::uikit::ui_font::{
    UILineBreakMode, UILineBreakModeTailTruncation, UITextAlignment, UITextAlignmentCenter,
    UITextAlignmentLeft, UITextAlignmentRight,
};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr,
};

pub struct UILabelHostObject {
    superclass: super::UIViewHostObject,
    /// `NSString*`
    text: id,
    /// `UIFont*`
    font: id,
    /// `UIColor*`
    text_color: id,
    text_alignment: UITextAlignment,
    line_break_mode: UILineBreakMode,
    number_of_lines: NSInteger,
}
impl_HostObject_with_superclass!(UILabelHostObject);
impl Default for UILabelHostObject {
    fn default() -> Self {
        UILabelHostObject {
            superclass: Default::default(),
            text: nil,
            font: nil,
            text_color: nil,
            text_alignment: UITextAlignmentLeft,
            line_break_mode: UILineBreakModeTailTruncation,
            number_of_lines: 1,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UILabel: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UILabelHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder: coder];
    // Use default values by calling the setters
    // TODO: Decode the actual property values from the coder
    () = msg![env; this setFont:nil];
    () = msg![env; this setTextColor:nil];
    () = msg![env; this setBackgroundColor:nil];
    // Built-in views don't have user-controlled opaqueness.
    () = msg_super![env; this setOpaque:false];
    this
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    // These aren't redundant, the setters fetch the real defaults.
    () = msg![env; this setFont:nil];
    () = msg![env; this setTextColor:nil];
    () = msg![env; this setBackgroundColor:nil];
    // Built-in views don't have user-controlled opaqueness.
    () = msg_super![env; this setOpaque:false];
    this
}

- (())dealloc {
    let &UILabelHostObject {
        superclass: _,
        text,
        font,
        text_color,
        text_alignment: _,
        line_break_mode: _,
        number_of_lines: _,
    } = env.objc.borrow(this);
    release(env, text);
    release(env, font);
    release(env, text_color);
    msg_super![env; this dealloc]
}

// TODO: initWithCoder:

- (id)text {
    env.objc.borrow::<UILabelHostObject>(this).text
}
- (())setText:(id)new_text { // NSString*
    let new_text: id = msg![env; new_text copy];
    let old_text = std::mem::replace(
        &mut env.objc.borrow_mut::<UILabelHostObject>(this).text,
        new_text
    );
    release(env, old_text);

    () = msg![env; this setNeedsDisplay];
}

- (id)font {
    env.objc.borrow::<UILabelHostObject>(this).font
}
- (())setFont:(id)new_font { // UIFont*
    let new_font: id = if new_font == nil {
        // reset to default
        let size: CGFloat = 17.0;
        msg_class![env; UIFont systemFontOfSize:size]
    } else {
        new_font
    };

    let old_font = std::mem::replace(
        &mut env.objc.borrow_mut::<UILabelHostObject>(this).font,
        new_font
    );
    retain(env, new_font);
    release(env, old_font);

    () = msg![env; this setNeedsDisplay];
}

- (id)textColor {
    env.objc.borrow::<UILabelHostObject>(this).text_color
}
- (())setTextColor:(id)new_text_color { // UIFont*
    let new_text_color: id = if new_text_color == nil {
        msg_class![env; UIColor blackColor]
    } else {
        new_text_color
    };

    let old_text_color = std::mem::replace(
        &mut env.objc.borrow_mut::<UILabelHostObject>(this).text_color,
        new_text_color
    );
    retain(env, new_text_color);
    release(env, old_text_color);

    () = msg![env; this setNeedsDisplay];
}

- (())setBackgroundColor:(id)color { // UIColor*
    // This overrides the standard setBackgroundColor: accessor on UIView.
    // UILabel seems to default to white, and setting the background color to
    // nil also just gives white, rather than the normal transparency. I don't
    // know how or why it does that, but overriding this setter seems like a
    // reasonable way to match that behavior.
    let color: id = if color == nil {
        msg_class![env; UIColor whiteColor]
    } else {
        color
    };
    msg_super![env; this setBackgroundColor:color]
}
- (())setOpaque:(bool)_opaque {
    // Built-in views don't have user-controlled opaqueness.
}

- (UITextAlignment)textAlignment {
    env.objc.borrow::<UILabelHostObject>(this).text_alignment
}
- (())setTextAlignment:(UITextAlignment)text_alignment { // UIFont*
    env.objc.borrow_mut::<UILabelHostObject>(this).text_alignment = text_alignment;
    () = msg![env; this setNeedsDisplay];
}

- (UILineBreakMode)lineBreakMode {
    env.objc.borrow::<UILabelHostObject>(this).line_break_mode
}
- (())setLineBreakMode:(UILineBreakMode)line_break_mode { // UIFont*
    env.objc.borrow_mut::<UILabelHostObject>(this).line_break_mode = line_break_mode;
    () = msg![env; this setNeedsDisplay];
}

- (NSInteger)numberOfLines {
    env.objc.borrow::<UILabelHostObject>(this).number_of_lines
}
- (())setNumberOfLines:(NSInteger)number {
    env.objc.borrow_mut::<UILabelHostObject>(this).number_of_lines = number;
    if number != 0 && number != 1 {
        log!("TODO: UILabel numberOfLines > 1 (label {:?})", this);
    }
    () = msg![env; this setNeedsDisplay];
}

- (())drawRect:(CGRect)_rect {
    let bounds: CGRect = msg![env; this bounds];
    let context = UIGraphicsGetCurrentContext(env);

    let &mut UILabelHostObject {
        superclass: _,
        text,
        font,
        text_color,
        text_alignment,
        line_break_mode,
        number_of_lines,
    } = env.objc.borrow_mut(this);

    let (r, g, b, a) = ui_color::get_rgba(&env.objc, text_color);
    CGContextSetRGBFillColor(env, context, r, g, b, a);

    // TODO: handle line counts other than 0 and 1 properly. 0 = unlimited
    // (note the log message in setNumberOfLines:)
    let single_line = number_of_lines == 1;

    let calculated_size: CGSize = if single_line {
        msg![env; text sizeWithFont:font]
    } else {
        msg![env; text sizeWithFont:font
                  constrainedToSize:(bounds.size)
                      lineBreakMode:line_break_mode]
    };

    // UILabel always vertically centers text
    // (TODO: check whether this is actually a UILabel thing, or a property of
    // UIStringDrawing?)
    let rect = CGRect {
        origin: CGPoint {
            x: bounds.origin.x,
            y: bounds.origin.y + (bounds.size.height - calculated_size.height) / 2.0,
        },
        size: CGSize {
            width: bounds.size.width,
            // This is necessary for when the calculated size is actually larger
            // than the bounds.
            height: calculated_size.height,
        },
    };

    let _size: CGSize = if single_line {
        let x_offset = match text_alignment {
            UITextAlignmentLeft => 0.0,
            UITextAlignmentCenter => 0.5,
            UITextAlignmentRight => 1.0,
            _ => unimplemented!(),
        };
        let point = CGPoint {
            x: rect.origin.x + x_offset * (bounds.size.width - calculated_size.width),
            y: rect.origin.y
        };
        msg![env; text drawAtPoint:point
                          withFont:font]
    } else {
        msg![env; text drawInRect:rect
                         withFont:font
                    lineBreakMode:line_break_mode
                        alignment:text_alignment]
    };
}

@end

};
