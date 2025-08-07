/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar`.

use crate::abi::{GuestArg, GuestRet};
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::{
    core_graphics::{CGFloat, CGPoint, CGRect, CGSize},
    foundation::{ns_array, NSUInteger},
};
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr, SEL,
};

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIBarStyle {
    UIBarStyleDefault,
    UIBarStyleBlack,
}

impl GuestArg for UIBarStyle {
    const REG_COUNT: usize = 1;

    fn from_regs(regs: &[u32]) -> Self {
        UIBarStyle::try_from(regs[0] as i32).unwrap_or(UIBarStyle::UIBarStyleDefault)
    }

    fn to_regs(self, regs: &mut [u32]) {
        regs[0] = self as i32 as u32;
    }
}

impl GuestRet for UIBarStyle {
    fn to_regs(self, regs: &mut [u32]) {
        regs[0] = self as i32 as u32;
    }
}

impl TryFrom<i32> for UIBarStyle {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UIBarStyle::UIBarStyleDefault),
            1 => Ok(UIBarStyle::UIBarStyleBlack),
            _ => Err(()),
        }
    }
}

pub struct UIToolbarHostObject {
    superclass: super::UIViewHostObject,
    items: Vec<id>,
    bar_style: UIBarStyle,
}
impl_HostObject_with_superclass!(UIToolbarHostObject);

impl Default for UIToolbarHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            items: Vec::new(),
            bar_style: UIBarStyle::UIBarStyleDefault,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIToolbar: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIToolbarHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (CGSize)sizeThatFits:(CGSize)size {
    // Tested with real iOS that height for Toolbar is 44.0
    CGSize { width: size.width, height: 44.0 }
}

- (id)initWithFrame:(CGRect)frame {
    let mut frame = frame;
    let current_frame_size = frame.size;

    // Frame height is usually set as 0.00 which is wrong
    // Use sizeThatFits to get the correct height
    let size = msg![env; this sizeThatFits:current_frame_size];
    frame.size = size;

    msg_super![env; this initWithFrame:frame]
}

- (())dealloc {
    let UIToolbarHostObject {
        superclass: _,
        items,
        bar_style: _,
    } = std::mem::take(env.objc.borrow_mut(this));

    for item in items {
        release(env, item);
    }

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
            retain(env, next);

            // Add view to toolbar
            let label: id = msg![env; next label];
            if label != nil {
                () = msg![env; this addSubview:label];
            }

            tmp_items.push(next);
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

    for (i, item) in items.into_iter().enumerate() {
        let label: id = msg![env; item label];
        if label == nil {
            continue;
        }

        let frame: CGRect = msg![env; label frame];

        if location.x >= frame.origin.x
            && location.x <= frame.origin.x + frame.size.width
            && location.y >= frame.origin.y
            && location.y <= frame.origin.y + frame.size.height
        {
            log!("UIToolbar detected tap on item {}", i);

            let target: id = msg![env; item target];
            let action: SEL = msg![env; item action];
            if target != nil {
                () = msg![env; this sendAction:action to:target forEvent:event];
            }
            break;
        }
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

- (())setBarStyle:(UIBarStyle)style {
    env.objc.borrow_mut::<UIToolbarHostObject>(this).bar_style = style;

    () = msg![env; this setNeedsDisplay];
}

- (UIBarStyle)barStyle {
    env.objc.borrow::<UIToolbarHostObject>(this).bar_style
}

- (())drawRect:(CGRect)_rect {
    let bounds: CGRect = msg![env; this bounds];

    let bg_color: id = msg_class![env; UIColor darkGrayColor];
    let item_bg_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; this setBackgroundColor:bg_color];

    let items = env.objc.borrow::<UIToolbarHostObject>(this).items.to_vec();
    let spacing: CGFloat = 8.0;

    let mut total_width: CGFloat = 0.0;
    let mut item_sizes: Vec<CGSize> = Vec::new();

    let items_copy = items.clone();

    for item in items {
        let label: id = msg![env; item label];
        if label == nil {
            item_sizes.push(CGSize { width: 0.0, height: 0.0 });
            continue;
        }

        () = msg![env; label sizeToFit];
        let label_frame: CGRect = msg![env; label frame];
        let label_size = label_frame.size;

        let padded_width = label_size.width + spacing * 2.0;
        let padded_height = bounds.size.height - spacing;

        item_sizes.push(CGSize {
            width: padded_width,
            height: padded_height,
        });

        total_width += padded_width;
    }

    let item_count = item_sizes.len();
    if item_count == 0 {
        return;
    }

    total_width += spacing * 2.0;
    let total_spacing = bounds.size.width - total_width;
    let item_spacing = total_spacing / ((item_count - 1) as f32);

    let mut x = spacing;
    let y = bounds.origin.y + spacing / 2.0;

    for (i, size) in item_sizes.into_iter().enumerate() {
        let item = items_copy[i];
        let label: id = msg![env; item label];
        if label == nil {
            continue;
        }

        let frame = CGRect {
            origin: CGPoint { x, y },
            size,
        };

        () = msg![env; label setFrame:frame];
        () = msg![env; label setBackgroundColor:item_bg_color];
        () = msg![env; label setTextAlignment:UITextAlignmentCenter];

        let layer: id = msg![env; label layer];
        () = msg![env; layer setCornerRadius:(10.0 as CGFloat)];

        x += size.width + item_spacing;
    }

}

@end

};
