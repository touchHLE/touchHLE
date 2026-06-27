/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar` and related bar-style types.
//!
//! Supports `UIBarButtonItem` objects with optional `customView`,
//! flexible/fixed space items, `barStyle`, `isTranslucent`, `tintColor`,
//! `barTintColor`, and a weak `delegate` reference.
//!
//! `UIBarButtonItem` is implemented as a `UIControl` subclass in this
//! emulator, so items without a `customView` are added directly as
//! subviews of the toolbar.  Items that *do* have a `customView` use
//! that view as the subview instead.

use crate::abi::{GuestArg, GuestRet};
use crate::frameworks::uikit::ui_view::ui_control::ui_bar_button_item::UIBarButtonSystemItem;
use crate::frameworks::{
    core_graphics::{CGFloat, CGPoint, CGRect, CGSize},
    foundation::{ns_array, NSUInteger},
};
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr,
};

// ---------------------------------------------------------------------------
// UIBarStyle
// ---------------------------------------------------------------------------

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIBarStyle {
    UIBarStyleDefault = 0,
    UIBarStyleBlack = 1,
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
    fn from_regs(regs: &[u32]) -> Self {
        UIBarStyle::try_from(regs[0] as i32).unwrap_or(UIBarStyle::UIBarStyleDefault)
    }

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

// ---------------------------------------------------------------------------
// Host object
// ---------------------------------------------------------------------------

pub struct UIToolbarHostObject {
    superclass: super::UIViewHostObject,
    /// The logical `UIBarButtonItem` array (not necessarily UIView objects).
    /// Retains one reference per slot.
    items: Vec<id>,
    /// Actual subviews, parallel to `items`.
    /// - If the item has a `customView`, this is the `customView`.
    /// - Otherwise this is the item itself (UIBarButtonItem IS-A UIControl
    ///   in this emulator).
    /// Retains one reference per slot independently of `items`.
    button_views: Vec<id>,
    bar_style: UIBarStyle,
    /// Whether the toolbar background is translucent (default: true).
    is_translucent: bool,
    /// Weak reference — not retained, matches UIKit convention.
    delegate: id,
}
impl_HostObject_with_superclass!(UIToolbarHostObject);

impl Default for UIToolbarHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            items: Vec::new(),
            button_views: Vec::new(),
            bar_style: UIBarStyle::UIBarStyleDefault,
            is_translucent: true,
            delegate: nil,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: resolve the subview for a UIBarButtonItem.
// ---------------------------------------------------------------------------

/// Return the view that should be placed as a subview of the toolbar for
/// the given `UIBarButtonItem`.  If the item has a `customView` that is not
/// nil, that view is returned; otherwise the item itself is returned (it
/// descends from `UIControl` in this emulator and renders itself).
fn button_view_for_item(env: &mut crate::Environment, item: id) -> id {
    let custom: id = msg![env; item customView];
    if custom != nil {
        custom
    } else {
        item
    }
}

/// Apply a background color with the appropriate alpha for the given style
/// and translucency flag.
fn apply_toolbar_background(
    env: &mut crate::Environment,
    this: id,
    style: UIBarStyle,
    is_translucent: bool,
) {
    let (base_color, opaque_alpha, translucent_alpha): (id, CGFloat, CGFloat) = match style {
        UIBarStyle::UIBarStyleDefault => {
            let c: id = msg_class![env; UIColor darkGrayColor];
            (c, 1.0, 0.85)
        }
        UIBarStyle::UIBarStyleBlack => {
            let c: id = msg_class![env; UIColor blackColor];
            (c, 1.0, 0.90)
        }
    };
    let alpha = if is_translucent {
        translucent_alpha
    } else {
        opaque_alpha
    };
    let colored: id = msg![env; base_color colorWithAlphaComponent:alpha];
    () = msg![env; this setBackgroundColor:colored];
}

// ---------------------------------------------------------------------------
// Class implementation
// ---------------------------------------------------------------------------

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIToolbar: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<UIToolbarHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

// MARK: Initialisation

- (CGSize)sizeThatFits:(CGSize)size {
    // Standard iOS toolbar height is 44 pt.
    CGSize { width: size.width, height: 44.0 }
}

- (id)initWithFrame:(CGRect)frame {
    let mut frame = frame;
    // Games sometimes pass height 0; snap to the correct height.
    // Note: cannot use frame.size directly inside msg![] — extract first.
    let current_size = frame.size;
    let corrected: CGSize = msg![env; this sizeThatFits:current_size];
    frame.size = corrected;

    let this: id = msg_super![env; this initWithFrame:frame];
    () = msg![env; this setOpaque:false];

    // Default appearance: translucent dark-gray.
    apply_toolbar_background(
        env, this,
        UIBarStyle::UIBarStyleDefault,
        true,
    );
    this
}

// MARK: Deallocation

- (())dealloc {
    let UIToolbarHostObject {
        superclass: _,
        items,
        button_views,
        bar_style: _,
        is_translucent: _,
        delegate: _,  // weak — not released
    } = std::mem::take(env.objc.borrow_mut(this));

    // Release button_views first (they may be the same objects as items or
    // may be separate customView objects).
    for bv in button_views {
        release(env, bv);
    }
    for item in items {
        release(env, item);
    }

    msg_super![env; this dealloc]
}

// MARK: Items

- (())setItems:(id)items {
    () = msg![env; this setItems:items animated:false];
}

- (())setItems:(id)new_items animated:(bool)_animated {
    // Animation is not implemented; we lay out immediately.

    let count: NSUInteger = if new_items != nil {
        msg![env; new_items count]
    } else {
        0
    };

    // ------------------------------------------------------------------
    // Build new item + view lists (retain both sides before touching the
    // old lists so reference counts stay valid throughout).
    // ------------------------------------------------------------------
    let mut tmp_items: Vec<id> = Vec::with_capacity(count as usize);
    let mut tmp_views: Vec<id> = Vec::with_capacity(count as usize);

    for i in 0..count {
        let item: id = msg![env; new_items objectAtIndex:i];
        retain(env, item);
        tmp_items.push(item);

        let bv = button_view_for_item(env, item);
        retain(env, bv);
        tmp_views.push(bv);
    }

    // ------------------------------------------------------------------
    // Tear down old lists.
    // ------------------------------------------------------------------
    let old_views = std::mem::take(&mut env.objc.borrow_mut::<UIToolbarHostObject>(this).button_views);
    let old_items = std::mem::take(&mut env.objc.borrow_mut::<UIToolbarHostObject>(this).items);

    for bv in old_views {
        () = msg![env; bv removeFromSuperview];
        release(env, bv);
    }
    for item in old_items {
        release(env, item);
    }

    // ------------------------------------------------------------------
    // Install new lists and add subviews.
    // ------------------------------------------------------------------
    env.objc.borrow_mut::<UIToolbarHostObject>(this).items      = tmp_items;
    env.objc.borrow_mut::<UIToolbarHostObject>(this).button_views =
        tmp_views.clone();

    for bv in tmp_views {
        () = msg![env; this addSubview:bv];
    }

    () = msg![env; this setNeedsLayout];
}

- (id)items {
    let items =
        env.objc.borrow::<UIToolbarHostObject>(this).items.to_vec();
    for &item in &items {
        retain(env, item);
    }
    let arr = ns_array::from_vec(env, items);
    autorelease(env, arr)
}

// MARK: Layout

- (())layoutSubviews {
    () = msg_super![env; this layoutSubviews];

    let bounds: CGRect = msg![env; this bounds];
    let host = env.objc.borrow::<UIToolbarHostObject>(this);
    let items = host.items.to_vec();
    let views = host.button_views.to_vec();

    if items.is_empty() {
        return;
    }

    // ------------------------------------------------------------------
    // Pass 1: compute a preferred width for every slot.
    // FlexibleSpace slots are recorded with 0 and resolved in pass 2.
    // ------------------------------------------------------------------
    let mut widths: Vec<CGFloat>  = Vec::with_capacity(items.len());
    let mut fixed_total: CGFloat  = 0.0;
    let mut flex_count:  usize    = 0;

    for &item in &items {
        // An explicit width > 0 always wins.
        let explicit_w: CGFloat = msg![env; item width];
        if explicit_w > 0.0 {
            widths.push(explicit_w);
            fixed_total += explicit_w;
            continue;
        }

        let sys: UIBarButtonSystemItem = msg![env; item systemItem];
        match sys {
            UIBarButtonSystemItem::FlexibleSpace => {
                flex_count += 1;
                widths.push(0.0); // resolved below
            }
            UIBarButtonSystemItem::FixedSpace => {
                // Default iOS fixed-space gap between button groups.
                let w: CGFloat = 6.0;
                widths.push(w);
                fixed_total += w;
            }
            _ => {
                let hint = bounds.size;
                let sz: CGSize = msg![env; item sizeThatFits:hint];
                widths.push(sz.width);
                fixed_total += sz.width;
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 2: distribute remaining width evenly among flexible slots.
    // ------------------------------------------------------------------
    if flex_count > 0 {
        let remaining = (bounds.size.width - fixed_total).max(0.0);
        let flex_w    = remaining / flex_count as CGFloat;
        for (i, &item) in items.iter().enumerate() {
            if widths[i] == 0.0 {
                let sys: UIBarButtonSystemItem =
                    msg![env; item systemItem];
                if sys == UIBarButtonSystemItem::FlexibleSpace {
                    widths[i] = flex_w;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 3: assign frames to the actual subviews.
    // ------------------------------------------------------------------
    let margin: CGFloat = 4.0;
    let h = (bounds.size.height - margin * 2.0).max(0.0);
    let y = margin;
    let mut x: CGFloat  = 0.0;

    for (i, &bv) in views.iter().enumerate() {
        let w = widths[i];
        let frame = CGRect {
            origin: CGPoint { x, y },
            size:   CGSize  { width: w, height: h },
        };
        () = msg![env; bv setFrame:frame];
        x += w;
    }
}

- (())drawRect:(CGRect)_rect {
    // Background is set in initWithFrame: / setBarStyle: / setTranslucent:.
}

// MARK: Bar style

- (())setBarStyle:(UIBarStyle)style {
    let translucent =
        env.objc.borrow::<UIToolbarHostObject>(this).is_translucent;
    env.objc.borrow_mut::<UIToolbarHostObject>(this).bar_style = style;
    apply_toolbar_background(env, this, style, translucent);
    () = msg![env; this setNeedsDisplay];
}

- (UIBarStyle)barStyle {
    env.objc.borrow::<UIToolbarHostObject>(this).bar_style
}

// MARK: Translucency

- (())setTranslucent:(bool)translucent {
    let style =
        env.objc.borrow::<UIToolbarHostObject>(this).bar_style;
    env.objc.borrow_mut::<UIToolbarHostObject>(this).is_translucent =
        translucent;
    apply_toolbar_background(env, this, style, translucent);
    () = msg![env; this setNeedsDisplay];
}

- (bool)isTranslucent {
    env.objc.borrow::<UIToolbarHostObject>(this).is_translucent
}

// MARK: Tint / bar tint color

// Forward tintColor to UIView (propagates to button items).
- (())setTintColor:(id)color {
    () = msg_super![env; this setTintColor:color];
    // Re-layout so items can react to the new tint.
    () = msg![env; this setNeedsLayout];
    () = msg![env; this setNeedsDisplay];
}

// barTintColor overrides the entire toolbar background color.
- (())setBarTintColor:(id)color {
    if color != nil {
        () = msg![env; this setBackgroundColor:color];
        () = msg![env; this setNeedsDisplay];
    }
}

- (id)barTintColor {
    // Expose the current background color as barTintColor.
    msg![env; this backgroundColor]
}

// MARK: Delegate (weak reference — not retained)

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UIToolbarHostObject>(this).delegate = delegate;
}

- (id)delegate {
    env.objc.borrow::<UIToolbarHostObject>(this).delegate
}

@end

};
