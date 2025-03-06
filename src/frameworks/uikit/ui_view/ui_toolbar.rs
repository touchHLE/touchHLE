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
    release, retain, ClassExports, NSZonePtr,
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
        let mut tmp_stack: Vec<id> = Vec::new();
        let count: NSUInteger = msg![env; items count];
        // TODO: zero count
        assert!(count > 0);
        for i in 0..(count - 1) {
            let next: id = msg![env; items objectAtIndex:i];
            tmp_stack.push(next);
            retain(env, next);
        }
        env.objc.borrow_mut::<UIToolbarHostObject>(this).items = tmp_stack;
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
    }

    - (id)barStyle {
        env.objc.borrow::<UIToolbarHostObject>(this).bar_style
    }

    - (())drawRect:(CGRect)_rect {
        // I think this is implimented correctly
        // the toolbar does not actually render
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
            let title_size: crate::frameworks::core_graphics::CGSize = msg![env; title sizeWithFont:font];

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
