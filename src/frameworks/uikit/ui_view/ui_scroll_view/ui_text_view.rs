/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITextView`.

use crate::frameworks::core_graphics::cg_context::CGContextSetRGBFillColor;
use crate::frameworks::core_graphics::cg_geometry::CGPointZero;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::uikit::ui_color;
use crate::frameworks::uikit::ui_font::{
    UILineBreakModeTailTruncation, UITextAlignment, UITextAlignmentLeft,
};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr,
};
use crate::Environment;

pub struct UITextViewHostObject {
    superclass: super::UIScrollViewHostObject,
    editable: bool,
    /// `NSString*`
    text: id,
    /// `UIFont*`
    font: id,
    /// `UIColor*`
    text_color: id,
    text_alignment: UITextAlignment,
}
impl_HostObject_with_superclass!(UITextViewHostObject);
impl Default for UITextViewHostObject {
    fn default() -> Self {
        UITextViewHostObject {
            superclass: Default::default(),
            editable: false,
            font: nil,
            text: nil,
            text_color: nil,
            text_alignment: UITextAlignmentLeft,
        }
    }
}

// Update contentOffset and contentSize when anything that potentially affects
// contentSize like font and text change.
fn update_scroll(env: &mut Environment, this: id) {
    let bounds: CGRect = msg![env; this bounds];
    let bound_size = bounds.size;
    let font: id = msg![env; this font];
    let text: id = msg![env; this text];

    // Calculate our new contentSize
    let calculated_size: CGSize = msg![env; text sizeWithFont:font constrainedToSize:bound_size];
    () = msg![env; this setContentSize:calculated_size];

    // Reset contentOffset if we have now gone out of bounds of contentSize,
    // otherwise ignore.
    let current_content_offset: CGPoint = msg![env; this contentOffset];
    if current_content_offset.x > calculated_size.width - bounds.size.width
        || current_content_offset.y > calculated_size.height - bounds.size.height
    {
        () = msg![env; this setContentOffset:(CGPoint { x: 0.0, y: 0.0 })];
    }
}
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITextView: UIScrollView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITextViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    // These aren't redundant, the setters fetch the real defaults.
    () = msg![env; this setFont:nil];
    () = msg![env; this setTextColor:nil];
    // TODO: support background color
    //() = msg![env; this setBackgroundColor:nil];
    this
}

- (())dealloc {
    let UITextViewHostObject {
        superclass: _,
        editable: _,
        font,
        text,
        text_color,
        text_alignment: _
    } = std::mem::take(env.objc.borrow_mut(this));

    release(env, font);
    release(env, text_color);
    release(env, text);
    msg_super![env; this dealloc]
}

- (id)text {
    env.objc.borrow::<UITextViewHostObject>(this).text
}
- (())setText:(id)new_text { // NSString*
    let hostobj  = env.objc.borrow_mut::<UITextViewHostObject>(this);
    let old_text = std::mem::replace(&mut hostobj.text, new_text);
    retain(env, new_text);
    release(env, old_text);
    update_scroll(env,this);
    () = msg![env; this setNeedsDisplay];
}

- (id)textColor {
    env.objc.borrow::<UITextViewHostObject>(this).text_color
}
- (())setTextColor:(id)new_text_color { // UIColor*
    let new_text_color: id = if new_text_color == nil {
        msg_class![env; UIColor whiteColor]
    } else {
        new_text_color
    };

    let hostobj  = env.objc.borrow_mut::<UITextViewHostObject>(this);
    let old_text_color = std::mem::replace(&mut hostobj.text_color, new_text_color);
    retain(env, new_text_color);
    release(env, old_text_color);
    () = msg![env; this setNeedsDisplay];
}

- (UITextAlignment)textAlignment {
    env.objc.borrow::<UITextViewHostObject>(this).text_alignment
}
- (())setTextAlignment:(UITextAlignment)new_text_alignment {
    env.objc.borrow_mut::<UITextViewHostObject>(this).text_alignment = new_text_alignment;
    () = msg![env; this setNeedsDisplay];
}

- (id)font {
    env.objc.borrow::<UITextViewHostObject>(this).font
}
- (())setFont:(id)new_font { // UIFont*
    let new_font: id = if new_font == nil {
        // reset to default
        let size: CGFloat = 17.0;
        msg_class![env; UIFont systemFontOfSize:size]
    } else {
        new_font
    };

    let hostobj  = env.objc.borrow_mut::<UITextViewHostObject>(this);
    let old_font = std::mem::replace(&mut hostobj.font, new_font);
    retain(env, new_font);
    release(env, old_font);
    update_scroll(env,this);
    () = msg![env; this setNeedsDisplay];

}

// TODO: Make editable actually do something
- (bool)isEditable {
    env.objc.borrow::<UITextViewHostObject>(this).editable
}
- (())setEditable:(bool)editable {
    env.objc.borrow_mut::<UITextViewHostObject>(this).editable = editable;
}

- (())drawRect:(CGRect)_rect {
    let bounds: CGRect = msg![env; this bounds];
    let context = UIGraphicsGetCurrentContext(env);

    let &mut UITextViewHostObject {
        superclass: _,
        editable: _,
        font,
        text,
        text_color,
        text_alignment
    } = env.objc.borrow_mut(this);

    let (r, g, b, a) = ui_color::get_rgba(&env.objc, text_color);
    CGContextSetRGBFillColor(env, context, r, g, b, a);

    let content_offset: CGPoint = msg![env; this contentOffset];
    let rect = CGRect {
        origin: CGPointZero,
        // If size is not expanded by the offset,
        // the text is rendered truncated.
        size: CGSize {
            width: bounds.size.width + content_offset.x,
            height: bounds.size.height + content_offset.y,
        }
    };

    log_dbg!("UItextView text rendering in rect {:?}", rect);
    let _size: CGSize = msg![env; text drawInRect:rect
                                         withFont:font
                                    lineBreakMode:UILineBreakModeTailTruncation
                                        alignment:text_alignment];
}

@end

};
