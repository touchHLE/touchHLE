/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar`.

use crate::frameworks::core_graphics::cg_context::{CGContextFillRect, CGContextSetRGBFillColor};
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::{ns_array, NSUInteger};
use crate::frameworks::uikit::ui_graphics::UIGraphicsGetCurrentContext;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr, SEL,
};

/// Holds the ivars for UIToolbar instances.
#[derive(Default)]
pub struct UIToolbarHostObject {
    superclass: super::UIViewHostObject,
    /// NSArray of UIBarButtonItems
    items: Vec<id>,
    /// NSNumber or primitive for style (mocked as id)
    bar_style: id,
}
impl_HostObject_with_superclass!(UIToolbarHostObject);

pub const CLASSES: ClassExports = objc_classes! {

    (env, this, _cmd);

    @implementation UIToolbar: UIView

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::<UIToolbarHostObject>::default();
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    - (id)initWithFrame:(CGRect)frame {
        log!("[(UIToolbar*){:?} initWithFrame:{:?}]", this, frame);

        let mut frame = frame;
        // Confirm frame height
        // For some reaon this can come in as o.oo
        // May be UIView sizeToFit
        if frame.size.height < 44.0 {
            frame.size.height = 44.0;
        }
        msg_super![env; this initWithFrame:frame]
    }

    - (())dealloc {
        let UIToolbarHostObject {
            superclass: _,
            items: _,
            bar_style,
        } = std::mem::take(env.objc.borrow_mut(this));

        release(env, bar_style);
        msg_super![env; this dealloc]
    }

    - (())setItems:(id)items { // NSArray *
        msg![env; this setItems:items animated:false]
    }

- (())setItems:(id)items // NSArray *
                animated:(bool)animated {
        assert!(!animated);
        let mut tmp_items: Vec<id> = Vec::new();
        let count: NSUInteger = msg![env; items count];
        if count == 0 {
            env.objc.borrow_mut::<UIToolbarHostObject>(this).items.clear();
            return;
        }


        // Filter and store only UIBarButtonItems with a non-nil target
        for i in 0..count {
            let next: id = msg![env; items objectAtIndex:i];
            let target: id = msg![env; next target];

            if target != nil {
                tmp_items.push(next);
                retain(env, next);
            }
        }

        env.objc.borrow_mut::<UIToolbarHostObject>(this).items = tmp_items;
        () = msg![env; this setNeedsDisplay];
    }

   - (())touchesEnded:(id)touches withEvent:(id)event {
        let items = env.objc.borrow::<UIToolbarHostObject>(this).items.to_vec();
        if items.is_empty() {
            return;
        }

        let touch: id = msg![env; touches anyObject];
        let location: CGPoint = msg![env; touch locationInView:this];

        let bounds: CGRect = msg![env; this bounds];
        let item_width = bounds.size.width / (items.len() as f32);

        let index = (location.x / item_width).floor() as usize;
        if index >= items.len() {
            return;
        }

        let item = items[index];

        log!("UIToolbar detected tap on item {}", index);

        // Forward the touch to the UIBarButtonItem!
        let target: id = msg![env; item target];
        let action: SEL = msg![env; item action];
        if target != nil {
            () = msg![env; this sendAction:action to:target forEvent:event];
        }

    }

    - (())sendAction:(SEL)action to:(id)target forEvent:(id)_event {
        if target == nil {
            log!("sendAction called with nil target!");
            return;
        }

        let _: id = msg![env; target performSelector:action];
    }

    - (id)items {
        let vcs = env.objc.borrow::<UIToolbarHostObject>(this).items.to_vec();
        for vc in &vcs {
            retain(env, *vc);
        }
        let res = ns_array::from_vec(env, vcs);
        autorelease(env, res)
    }

    - (())setBarStyle:(id)style {
        env.objc.borrow_mut::<UIToolbarHostObject>(this).bar_style = style;

        () = msg![env; this setNeedsDisplay];
    }

    - (id)barStyle {
        env.objc.borrow::<UIToolbarHostObject>(this).bar_style
    }

    - (())drawRect:(CGRect)_rect {
        let bounds: CGRect = msg![env; this bounds];
        let context = UIGraphicsGetCurrentContext(env);

        // Draw toolbar background
        CGContextSetRGBFillColor(env, context, 0.85, 0.85, 0.85, 1.0);
        CGContextFillRect(env, context, bounds);

        // Get items array from our host object
        let items = env.objc.borrow_mut::<UIToolbarHostObject>(this).items.to_vec();
        let count = items.len();

        // Prepare for drawing text labels
        let font: id = msg_class![env; UIFont systemFontOfSize:17_f32];
        let text_color: id = msg_class![env; UIColor blackColor];

        // Compute width for each item (simple equal division)
        let item_width = bounds.size.width / (count as f32);
        let item_height = bounds.size.height;

        let mut i = 0;
        for item in items {
            let title: id = msg![env; item title];
            if title == nil {
                continue;
            }

            // Compute item rect
            let x = bounds.origin.x + (i as f32) * item_width;
            let y = bounds.origin.y;
            let item_rect = CGRect {
                origin: CGPoint { x, y },
                size: CGSize {
                    width: item_width,
                    height: item_height,
                },
            };

            // Draw button background (lighter gray rectangle for button)
            CGContextSetRGBFillColor(env, context, 0.95, 0.95, 0.95, 1.0);
            CGContextFillRect(env, context, item_rect);

            // Draw title centered in button
            let title_size: CGSize = msg![env; title sizeWithFont:font];

            let title_point = CGPoint {
                x: item_rect.origin.x + (item_width - title_size.width) / 2.0,
                y: item_rect.origin.y + (item_height - title_size.height) / 2.0,
            };

            () = msg![env; text_color set];
            let _: CGSize = msg![env; title drawAtPoint:title_point withFont:font];
            i += 1;
        }
    }

    @end
};
