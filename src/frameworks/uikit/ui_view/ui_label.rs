/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UILabel`.

use super::{UIViewHostObject, UIViewSubclass};
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
    id, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports, ObjC,
};

pub struct UILabelData {
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
impl UILabelData {
    fn borrow_mut(objc: &mut ObjC, ui_label: id) -> &mut Self {
        let host_obj = objc.borrow_mut::<UIViewHostObject>(ui_label);
        let UIViewSubclass::UILabel(ref mut data) = host_obj.subclass else {
            panic!();
        };
        data
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UILabel: UIView

- (id)init {
    let this: id = msg_super![env; this init];
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    host_obj.subclass = UIViewSubclass::UILabel(UILabelData {
        text: nil,
        font: nil,
        text_color: nil,
        text_alignment: UITextAlignmentLeft,
        line_break_mode: UILineBreakModeTailTruncation,
        number_of_lines: 1,
    });
    // These aren't redundant, the setters fetch the real defaults.
    () = msg![env; this setFont:nil];
    () = msg![env; this setTextColor:nil];
    () = msg![env; this setBackgroundColor:nil];
    // Built-in views don't have user-controlled opaqueness.
    () = msg_super![env; this setOpaque:false];
    this
}

- (())dealloc {
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    let subclass = std::mem::take(&mut host_obj.subclass);
    let UIViewSubclass::UILabel(data) = subclass else {
        panic!();
    };
    release(env, data.text);
    release(env, data.font);
    release(env, data.text_color);
    msg_super![env; this dealloc]
}

// TODO: initWithCoder:

- (id)text {
    UILabelData::borrow_mut(&mut env.objc, this).text
}
- (())setText:(id)new_text { // UIString*
    let old_text = std::mem::replace(
        &mut UILabelData::borrow_mut(&mut env.objc, this).text,
        new_text
    );
    retain(env, new_text);
    release(env, old_text);

    () = msg![env; this setNeedsDisplay];
}

- (id)font {
    UILabelData::borrow_mut(&mut env.objc, this).font
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
        &mut UILabelData::borrow_mut(&mut env.objc, this).font,
        new_font
    );
    retain(env, new_font);
    release(env, old_font);

    () = msg![env; this setNeedsDisplay];
}

- (id)textColor {
    UILabelData::borrow_mut(&mut env.objc, this).text_color
}
- (())setTextColor:(id)new_text_color { // UIFont*
    let new_text_color: id = if new_text_color == nil {
        msg_class![env; UIColor blackColor]
    } else {
        new_text_color
    };

    let old_text_color = std::mem::replace(
        &mut UILabelData::borrow_mut(&mut env.objc, this).text_color,
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
    UILabelData::borrow_mut(&mut env.objc, this).text_alignment
}
- (())setTextAlignment:(UITextAlignment)text_alignment { // UIFont*
    UILabelData::borrow_mut(&mut env.objc, this).text_alignment = text_alignment;
    () = msg![env; this setNeedsDisplay];
}

- (UILineBreakMode)lineBreakMode {
    UILabelData::borrow_mut(&mut env.objc, this).line_break_mode
}
- (())setLineBreakMode:(UILineBreakMode)line_break_mode { // UIFont*
    UILabelData::borrow_mut(&mut env.objc, this).line_break_mode = line_break_mode;
    () = msg![env; this setNeedsDisplay];
}

- (NSInteger)numberOfLines {
    UILabelData::borrow_mut(&mut env.objc, this).number_of_lines
}
- (())setNumberOfLines:(NSInteger)number {
    UILabelData::borrow_mut(&mut env.objc, this).number_of_lines = number;
    if number != 0 && number != 1 {
        log!("TODO: UILabel numberOfLines > 1 (label {:?})", this);
    }
    () = msg![env; this setNeedsDisplay];
}

- (())drawRect:(CGRect)_rect {
    let bounds: CGRect = msg![env; this bounds];
    let context = UIGraphicsGetCurrentContext(env);

    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    let &UIViewSubclass::UILabel(UILabelData {
        text,
        font,
        text_color,
        text_alignment,
        line_break_mode,
        number_of_lines,
    }) = &host_obj.subclass else {
        panic!();
    };

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
        size: bounds.size,
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
