/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar`.

use crate::frameworks::uikit::ui_bar_item::ui_bar_button_item::{
    UIBarButtonItemHostObject, UIBarButtonSystemItemFixedSpace, UIBarButtonSystemItemFlexibleSpace,
};
use crate::frameworks::{
    core_graphics::{CGFloat, CGPoint, CGRect, CGSize},
    foundation::{ns_array, NSInteger, NSUInteger},
};
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr,
};

pub type UIBarStyle = NSInteger;

pub const UIBarStyleDefault: UIBarStyle = 0;
pub const UIBarStyleBlack: UIBarStyle = 1;
pub const UIBarStyleBlackTranslucent: UIBarStyle = 2;

pub struct UIToolbarHostObject {
    superclass: super::UIViewHostObject,
    items: Vec<id>,
    /// Internal views (UIControl*) corresponding to each item,
    item_views: Vec<id>,
    bar_style: UIBarStyle,
}
impl_HostObject_with_superclass!(UIToolbarHostObject);

impl Default for UIToolbarHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            items: Vec::new(),
            item_views: Vec::new(),
            bar_style: UIBarStyleDefault,
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
    let size = msg![env; this sizeThatFits:current_frame_size];
    frame.size = size;

    let this: id = msg_super![env; this initWithFrame:frame];
    () = msg![env; this setOpaque:false];
    () = msg![env; this setBarStyle:UIBarStyleDefault];
    this
}

- (())dealloc {
    let UIToolbarHostObject {
        superclass: _,
        items,
        item_views,
        bar_style: _,
    } = std::mem::take(env.objc.borrow_mut(this));

    for item in items {
        release(env, item);
    }
    for view in item_views {
        release(env, view);
    }

    msg_super![env; this dealloc]
}

- (())setItems:(id)items { // NSArray *
    msg![env; this setItems:items animated:false]
}

- (())setItems:(id)items // NSArray *
            animated:(bool)animated {
    if animated {
        log_dbg!("TODO: UIToolbar setItems:animated: animation not supported, ignoring");
    }

    let count: NSUInteger = msg![env; items count];
    let mut tmp_items: Vec<id> = Vec::new();
    let mut tmp_views: Vec<id> = Vec::new();

    for i in 0..count {
        let next: id = msg![env; items objectAtIndex:i];
        retain(env, next);
        tmp_items.push(next);
        let view: id = msg![env; next customView];
        retain(env, view);
        tmp_views.push(view);
    }

    // Remove old item views from superview
    let host = env.objc.borrow_mut::<UIToolbarHostObject>(this);
    let old_items = std::mem::replace(&mut host.items, tmp_items);
    let old_views = std::mem::replace(&mut host.item_views, tmp_views);

    for view in old_views {
        () = msg![env; view removeFromSuperview];
        release(env, view);
    }
    for item in old_items {
        release(env, item);
    }

    // Add new item views as subviews
    let item_views = env.objc.borrow::<UIToolbarHostObject>(this).item_views.to_vec();
    for view in item_views {
        () = msg![env; this addSubview:view];
    }

    () = msg![env; this layoutSubviews];
    () = msg![env; this setNeedsDisplay];
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

    let bg_color: id = match style {
        UIBarStyleDefault => {
            let c: id = msg_class![env; UIColor darkGrayColor];
            msg![env; c colorWithAlphaComponent:(0.8 as CGFloat)]
        }
        UIBarStyleBlack => {
            let c: id = msg_class![env; UIColor blackColor];
            msg![env; c colorWithAlphaComponent:(0.9 as CGFloat)]
        }
        UIBarStyleBlackTranslucent => {
            let c: id = msg_class![env; UIColor blackColor];
            msg![env; c colorWithAlphaComponent:(0.5 as CGFloat)]
        }
        _ => unimplemented!("UIBarStyle {}", style),
    };
    () = msg![env; this setBackgroundColor:bg_color];
}

- (UIBarStyle)barStyle {
    env.objc.borrow::<UIToolbarHostObject>(this).bar_style
}

- (())setTranslucent:(bool)translucent {
    log!("TODO: setTranslucent:{} ignored", translucent);
}

- (bool)isTranslucent {
    log!("TODO: isTranslucent returning false");
    false
}

- (())setFrame:(CGRect)frame {
    () = msg_super![env; this setFrame:frame];

    // UIView's addSubview: doesn't trigger a layout pass on the new subview,
    // so toolbars loaded from NIBs lay out once at zero width and never again.
    // Re-running layout on frame changes covers the case where the parent
    // sizes the toolbar after mounting it.
    () = msg![env; this layoutSubviews];
}

- (())layoutSubviews {
    let bounds: CGRect = msg![env; this bounds];
    let host = env.objc.borrow::<UIToolbarHostObject>(this);
    let items = host.items.to_vec();
    let item_views = host.item_views.to_vec();
    if items.is_empty() {
        return;
    }

    let mut total_fixed_width: CGFloat = 0.0;
    let mut item_widths: Vec<CGFloat> = Vec::new();
    let mut flexible_space_count: usize = 0;

    for &item in &items {
        let width: CGFloat = msg![env; item width];
        if width > 0.0 {
            item_widths.push(width);
            total_fixed_width += width;
        } else {
            let system_item = env.objc.borrow::<UIBarButtonItemHostObject>(item).system_item;
            if system_item == Some(UIBarButtonSystemItemFlexibleSpace) {
                flexible_space_count += 1;
                item_widths.push(0.0);
            } else if system_item == Some(UIBarButtonSystemItemFixedSpace) {
                let fixed_width = 42.0;
                item_widths.push(fixed_width);
                total_fixed_width += fixed_width;
            } else {
                let bounds_size = bounds.size;
                let size: CGSize = msg![env; item sizeThatFits:bounds_size];
                item_widths.push(size.width);
                total_fixed_width += size.width;
            }
        }
    }

    let item_count = items.len();
    let remaining_width = bounds.size.width - total_fixed_width;

    // When no FlexibleSpace items are present, distribute slack between items
    // so the first/last items sit flush with the toolbar edges. With a single
    // item, center it in the toolbar.
    let (flexible_width, gap, start_x) = if flexible_space_count > 0 {
        ((remaining_width / (flexible_space_count as f32)).max(0.0), 0.0, 0.0)
    } else if item_count > 1 {
        (0.0, (remaining_width / ((item_count - 1) as f32)).max(0.0), 0.0)
    } else {
        (0.0, 0.0, (remaining_width / 2.0).max(0.0))
    };

    let mut x = start_x;
    let button_margin: CGFloat = 4.0;
    let h = (bounds.size.height - button_margin * 2.0).max(0.0);
    let y = button_margin;

    for (i, &item) in items.iter().enumerate() {
        let mut w = item_widths[i];
        if w == 0.0 {
            let system_item = env.objc.borrow::<UIBarButtonItemHostObject>(item).system_item;
            if system_item == Some(UIBarButtonSystemItemFlexibleSpace) {
                w = flexible_width;
            }
        }

        let frame = CGRect {
            origin: CGPoint { x, y },
            size: CGSize { width: w, height: h },
        };

        // Position the item's internal view, then fill its label to bounds.
        let item_view = item_views[i];
        () = msg![env; item_view setFrame:frame];
        let label: id = msg![env; item label];
        if label != nil {
            let view_bounds: CGRect = msg![env; item_view bounds];
            () = msg![env; label setFrame:view_bounds];
        }

        x += w + gap;
    }
}

@end

};
