/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIButton`.

use super::{UIControlState, UIControlStateNormal};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_font::{
    UITextAlignmentCenter, UITextAlignmentLeft, UITextAlignmentRight,
};
use crate::mem::SafeRead;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;
use std::collections::HashMap;

type UIButtonType = NSInteger;
pub const UIButtonTypeCustom: UIButtonType = 0;
pub const UIButtonTypeRoundedRect: UIButtonType = 1;
const UIButtonTypeDetailDisclosure: UIButtonType = 2;
const UIButtonTypeInfoLight: UIButtonType = 3;
const UIButtonTypeInfoDark: UIButtonType = 4;
const UIButtonTypeContactAdd: UIButtonType = 5;

// MARK: - Content alignment types

type UIControlContentHorizontalAlignment = NSInteger;
const UIControlContentHorizontalAlignmentCenter: UIControlContentHorizontalAlignment = 0;
const UIControlContentHorizontalAlignmentLeft: UIControlContentHorizontalAlignment = 1;
const UIControlContentHorizontalAlignmentRight: UIControlContentHorizontalAlignment = 2;
const UIControlContentHorizontalAlignmentFill: UIControlContentHorizontalAlignment = 3;

type UIControlContentVerticalAlignment = NSInteger;
const UIControlContentVerticalAlignmentCenter: UIControlContentVerticalAlignment = 0;
const UIControlContentVerticalAlignmentTop: UIControlContentVerticalAlignment = 1;
const UIControlContentVerticalAlignmentBottom: UIControlContentVerticalAlignment = 2;
const UIControlContentVerticalAlignmentFill: UIControlContentVerticalAlignment = 3;

// MARK: - Edge insets

#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub struct UIEdgeInsets {
    pub top: CGFloat,
    pub left: CGFloat,
    pub bottom: CGFloat,
    pub right: CGFloat,
}
unsafe impl SafeRead for UIEdgeInsets {}

// MARK: - Host objects

#[derive(Default)]
struct UIButtonContentHostObject {
    /// `NSString*`
    title: id,
    /// `UIColor*`
    title_color: id,
    /// `UIImage*`
    image: id,
    /// `UIImage*`
    background_image: id,
}
impl HostObject for UIButtonContentHostObject {}

pub struct UIButtonHostObject {
    superclass: super::UIControlHostObject,
    type_: UIButtonType,
    /// `UILabel*`
    title_label: id,
    /// `UIImageView*`
    image_view: id,
    /// `UIImageView*`
    background_image_view: id,
    /// Values are `NSString*`
    titles_for_states: HashMap<UIControlState, id>,
    /// Values are `UIColor*`
    title_colors_for_states: HashMap<UIControlState, id>,
    /// Values are `UIImage*`
    images_for_states: HashMap<UIControlState, id>,
    /// Values are `UIImage*`
    background_images_for_states: HashMap<UIControlState, id>,
    content_horizontal_alignment: UIControlContentHorizontalAlignment,
    content_vertical_alignment: UIControlContentVerticalAlignment,
    content_edge_insets: UIEdgeInsets,
    title_edge_insets: UIEdgeInsets,
    image_edge_insets: UIEdgeInsets,
    adjusts_image_when_highlighted: bool,
    adjusts_image_when_disabled: bool,
    shows_touch_when_highlighted: bool,
    reverses_title_shadow_when_highlighted: bool,
    tint_color: id,
}
impl_HostObject_with_superclass!(UIButtonHostObject);

impl Default for UIButtonHostObject {
    fn default() -> Self {
        UIButtonHostObject {
            superclass: Default::default(),
            type_: UIButtonTypeCustom,
            title_label: nil,
            image_view: nil,
            background_image_view: nil,
            titles_for_states: HashMap::new(),
            title_colors_for_states: HashMap::new(),
            images_for_states: HashMap::new(),
            background_images_for_states: HashMap::new(),
            content_horizontal_alignment: UIControlContentHorizontalAlignmentCenter,
            content_vertical_alignment: UIControlContentVerticalAlignmentCenter,
            content_edge_insets: UIEdgeInsets::default(),
            title_edge_insets: UIEdgeInsets::default(),
            image_edge_insets: UIEdgeInsets::default(),
            adjusts_image_when_highlighted: true,
            adjusts_image_when_disabled: true,
            shows_touch_when_highlighted: false,
            reverses_title_shadow_when_highlighted: false,
            tint_color: nil,
        }
    }
}

// MARK: - Helpers

fn update(env: &mut Environment, this: id) {
    let title_label: id = msg![env; this titleLabel];
    let title: id = msg![env; this currentTitle];
    () = msg![env; title_label setText:title];
    let title_color: id = msg![env; this currentTitleColor];
    () = msg![env; title_label setTextColor:title_color];
    let image_view: id = msg![env; this imageView];
    let image: id = msg![env; this currentImage];

    () = msg![env; image_view setImage:image];
    let background_image_view: id = msg![env; this backgroundImageView];
    let background_image: id = msg![env; this currentBackgroundImage];

    () = msg![env; background_image_view setImage:background_image];

    // Hide the title label when the button has a foreground image, since the
    // image already contains any necessary text (e.g. baked-in button labels).
    // Without proper side-by-side layout the title would overlap the image.
    let hide_title = image != nil;
    () = msg![env; title_label setHidden:hide_title];
}

fn init_common(env: &mut Environment, this: id) -> id {
    () = msg![env; this setOpaque:false];
    let bg_color: id = msg_class![env; UIColor clearColor];

    let title_label: id = msg_class![env; UILabel new];
    () = msg![env; title_label setBackgroundColor:bg_color];
    () = msg![env; title_label setTextAlignment:UITextAlignmentCenter];

    let text_color: id = msg_class![env; UIColor whiteColor];
    let image_view: id = msg_class![env; UIImageView new];
    let background_image_view: id = msg_class![env; UIImageView new];

    let host_obj = env.objc.borrow_mut::<UIButtonHostObject>(this);
    host_obj.title_label = title_label;
    host_obj.image_view = image_view;
    host_obj.background_image_view = background_image_view;
    host_obj.titles_for_states.insert(UIControlStateNormal, nil);
    host_obj
        .title_colors_for_states
        .insert(UIControlStateNormal, text_color);
    host_obj.images_for_states.insert(UIControlStateNormal, nil);
    host_obj
        .background_images_for_states
        .insert(UIControlStateNormal, nil);

    () = msg![env; this addSubview:background_image_view];
    () = msg![env; this addSubview:image_view];
    () = msg![env; this addSubview:title_label];
    update(env, this);
    this
}

fn set_type(env: &mut Environment, button: id, type_: UIButtonType) {
    match type_ {
        UIButtonTypeCustom => (),
        UIButtonTypeRoundedRect => {
            let bg_color: id = msg_class![env; UIColor whiteColor];
            () = msg![env; button setBackgroundColor:bg_color];
            let text_color: id = msg_class![env; UIColor blackColor];
            () = msg![env; button setTitleColor:text_color forState:UIControlStateNormal];
            let layer: id = msg![env; button layer];
            () = msg![env; layer setCornerRadius:(10.0 as CGFloat)];
        }
        UIButtonTypeDetailDisclosure | UIButtonTypeInfoLight | UIButtonTypeInfoDark => {
            // System "info" buttons (types 2, 3, 4).
            // Set the canonical iOS size (18×19) so the button exists on
            // screen and can receive taps.
            let bounds = CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize { width: 18.0, height: 19.0 },
            };
            () = msg![env; button setBounds:bounds];
            let title = get_static_str(env, "i");
            () = msg![env; button setTitle:title forState:UIControlStateNormal];
        }
        UIButtonTypeContactAdd => {
            // System "+" button (type 5). Canonical iOS size is 29×29.
            let bounds = CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize { width: 29.0, height: 29.0 },
            };
            () = msg![env; button setBounds:bounds];
            let title = get_static_str(env, "+");
            () = msg![env; button setTitle:title forState:UIControlStateNormal];
        }
        _ => {
            log!("Warning: Unhandled UIButtonType {} in set_type, treating as Custom", type_);
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIButton: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIButtonHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)buttonWithType:(UIButtonType)button_type {
    let button: id = msg_class![env; UIButton alloc];
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize { width: 0.0, height: 0.0 },
    };
    let button: id = msg![env; button initWithFrame:frame];

    set_type(env, button, button_type);

    autorelease(env, button)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    let bg_color: id = msg_class![env; UIColor clearColor];
    () = msg![env; this setBackgroundColor:bg_color];
    let this = init_common(env, this);
    set_type(env, this, UIButtonTypeCustom);
    this
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let this = init_common(env, this);
    let key_ns_string = get_static_str(env, "UIButtonType");
    let type_: i32 = msg![env; coder decodeIntForKey:key_ns_string];
    set_type(env, this, type_);

    let key_ns_string = get_static_str(env, "UIButtonStatefulContent");
    let dict: id = msg![env; coder decodeObjectForKey:key_ns_string];
    assert!(dict != nil);
    log_dbg!("UIButtonStatefulContent dict: {}", {
        let desc: id = msg![env; dict description];
        to_rust_string(env, desc)
    });

    // Iterate over all state keys in the stateful content dictionary
    let all_keys: id = msg![env; dict allKeys];
    let keys_count: NSUInteger = msg![env; all_keys count];
    for i in 0..keys_count {
        let key_obj: id = msg![env; all_keys objectAtIndex:i];
        let state_val: i64 = msg![env; key_obj longLongValue];
        let state = state_val as UIControlState;
        let button_content: id = msg![env; dict objectForKey:key_obj];
        if button_content == nil {
            continue;
        }

        let title: id = msg![env; button_content title];
        if title != nil {
            log_dbg!("UIButton initWithCoder: title {} for state {}", to_rust_string(env, title), state);
            () = msg![env; this setTitle:title forState:state];
        }
        let title_color: id = msg![env; button_content titleColor];
        if title_color != nil {
            () = msg![env; this setTitleColor:title_color forState:state];
        }
        let image: id = msg![env; button_content image];
        if image != nil {
            log_dbg!("UIButton initWithCoder: setting image {:?} for state {}", image, state);
            () = msg![env; this setImage:image forState:state];
        }
        let bg_image: id = msg![env; button_content backgroundImage];
        if bg_image != nil {
            log_dbg!("UIButton initWithCoder: setting backgroundImage {:?} for state {}", bg_image, state);
            () = msg![env; this setBackgroundImage:bg_image forState:state];
        }
    }

    () = msg![env; this layoutSubviews];
    update(env, this);
    this
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<UIButtonHostObject>(this));
    release(env, host.title_label);
    release(env, host.image_view);
    release(env, host.background_image_view);
    release(env, host.tint_color);
    for (_, v) in host.titles_for_states            { release(env, v); }
    for (_, v) in host.title_colors_for_states      { release(env, v); }
    for (_, v) in host.images_for_states            { release(env, v); }
    for (_, v) in host.background_images_for_states { release(env, v); }
    msg_super![env; this dealloc]
}

// MARK: - Layout

- (())layoutSubviews {
    let host_object = env.objc.borrow_mut::<UIButtonHostObject>(this);
    let title_label = host_object.title_label;
    let background_image_view = host_object.background_image_view;
    let image_view = host_object.image_view;
    let bounds: CGRect = msg![env; this bounds];

    () = msg![env; title_label setFrame:bounds];
    () = msg![env; background_image_view setFrame:bounds];
    () = msg![env; image_view setFrame:bounds];
}

- (CGRect)titleRectForContentRect:(CGRect)content_rect {
    content_rect
}

- (CGRect)imageRectForContentRect:(CGRect)_content_rect {
    CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size:   CGSize  { width: 0.0, height: 0.0 },
    }
}

- (CGRect)contentRectForBounds:(CGRect)bounds { bounds }
- (CGRect)backgroundRectForBounds:(CGRect)bounds { bounds }

// MARK: - Type

- (UIButtonType)buttonType {
    env.objc.borrow::<UIButtonHostObject>(this).type_
}

// MARK: - Subviews

- (id)titleLabel            { env.objc.borrow::<UIButtonHostObject>(this).title_label }
- (id)imageView             { env.objc.borrow::<UIButtonHostObject>(this).image_view }
- (id)backgroundImageView   { env.objc.borrow::<UIButtonHostObject>(this).background_image_view }

// MARK: - State forwarding

- (())setEnabled:(bool)enabled {
    () = msg_super![env; this setEnabled:enabled];
    update(env, this);
}
- (())setSelected:(bool)selected {
    () = msg_super![env; this setSelected:selected];
    update(env, this);
}
- (())setHighlighted:(bool)highlighted {
    () = msg_super![env; this setHighlighted:highlighted];
    update(env, this);
}

// MARK: - Font

- (())setFont:(id)font {
    let label = env.objc.borrow::<UIButtonHostObject>(this).title_label;
    () = msg![env; label setFont:font];
    update(env, this);
}

// MARK: - Content alignment

- (UIControlContentHorizontalAlignment)contentHorizontalAlignment {
    env.objc.borrow::<UIButtonHostObject>(this).content_horizontal_alignment
}

- (())setContentHorizontalAlignment:(UIControlContentHorizontalAlignment)alignment {
    env.objc.borrow_mut::<UIButtonHostObject>(this).content_horizontal_alignment = alignment;
    let label = env.objc.borrow::<UIButtonHostObject>(this).title_label;
    let text_alignment = match alignment {
        UIControlContentHorizontalAlignmentLeft  => UITextAlignmentLeft,
        UIControlContentHorizontalAlignmentRight => UITextAlignmentRight,
        _                                        => UITextAlignmentCenter,
    };
    () = msg![env; label setTextAlignment:text_alignment];
}

- (UIControlContentVerticalAlignment)contentVerticalAlignment {
    env.objc.borrow::<UIButtonHostObject>(this).content_vertical_alignment
}

- (())setContentVerticalAlignment:(UIControlContentVerticalAlignment)alignment {
    env.objc.borrow_mut::<UIButtonHostObject>(this).content_vertical_alignment = alignment;
}

// MARK: - Edge insets (Hacked to use CGRect for GuestArg/GuestRet
// compatibility)

- (CGRect)contentEdgeInsets {
    let insets = env.objc.borrow::<UIButtonHostObject>(this).content_edge_insets;
    CGRect {
        origin: CGPoint { x: insets.top, y: insets.left },
        size: CGSize { width: insets.bottom, height: insets.right },
    }
}
- (())setContentEdgeInsets:(CGRect)rect {
    let insets = UIEdgeInsets {
        top: rect.origin.x,
        left: rect.origin.y,
        bottom: rect.size.width,
        right: rect.size.height,
    };
    env.objc.borrow_mut::<UIButtonHostObject>(this).content_edge_insets = insets;
}

- (CGRect)titleEdgeInsets {
    let insets = env.objc.borrow::<UIButtonHostObject>(this).title_edge_insets;
    CGRect {
        origin: CGPoint { x: insets.top, y: insets.left },
        size: CGSize { width: insets.bottom, height: insets.right },
    }
}
- (())setTitleEdgeInsets:(CGRect)rect {
    let insets = UIEdgeInsets {
        top: rect.origin.x,
        left: rect.origin.y,
        bottom: rect.size.width,
        right: rect.size.height,
    };
    env.objc.borrow_mut::<UIButtonHostObject>(this).title_edge_insets = insets;
}

- (CGRect)imageEdgeInsets {
    let insets = env.objc.borrow::<UIButtonHostObject>(this).image_edge_insets;
    CGRect {
        origin: CGPoint { x: insets.top, y: insets.left },
        size: CGSize { width: insets.bottom, height: insets.right },
    }
}
- (())setImageEdgeInsets:(CGRect)rect {
    let insets = UIEdgeInsets {
        top: rect.origin.x,
        left: rect.origin.y,
        bottom: rect.size.width,
        right: rect.size.height,
    };
    env.objc.borrow_mut::<UIButtonHostObject>(this).image_edge_insets = insets;
}

// MARK: - Behaviour flags

- (bool)adjustsImageWhenHighlighted {
    env.objc.borrow::<UIButtonHostObject>(this).adjusts_image_when_highlighted
}
- (())setAdjustsImageWhenHighlighted:(bool)adjusts {
    env.objc.borrow_mut::<UIButtonHostObject>(this).adjusts_image_when_highlighted = adjusts;
}

- (bool)adjustsImageWhenDisabled {
    env.objc.borrow::<UIButtonHostObject>(this).adjusts_image_when_disabled
}
- (())setAdjustsImageWhenDisabled:(bool)adjusts {
    env.objc.borrow_mut::<UIButtonHostObject>(this).adjusts_image_when_disabled = adjusts;
}

- (bool)showsTouchWhenHighlighted {
    env.objc.borrow::<UIButtonHostObject>(this).shows_touch_when_highlighted
}
- (())setShowsTouchWhenHighlighted:(bool)shows {
    env.objc.borrow_mut::<UIButtonHostObject>(this).shows_touch_when_highlighted = shows;
}

- (bool)reversesTitleShadowWhenHighlighted {
    env.objc.borrow::<UIButtonHostObject>(this).reverses_title_shadow_when_highlighted
}
- (())setReversesTitleShadowWhenHighlighted:(bool)value {
    env.objc.borrow_mut::<UIButtonHostObject>(this)
        .reverses_title_shadow_when_highlighted = value;
}

// MARK: - Tint

- (id)tintColor {
    env.objc.borrow::<UIButtonHostObject>(this).tint_color
}
- (())setTintColor:(id)color {
    let old = env.objc.borrow::<UIButtonHostObject>(this).tint_color;
    release(env, old);
    retain(env, color);
    env.objc.borrow_mut::<UIButtonHostObject>(this).tint_color = color;
}

- (())setTitleShadowOffset:(CGSize)_offset {
    // Per Apple docs this is a deprecated visual property (UIButton.titleShadowOffset).
    // We accept and ignore it since touchHLE does not render title shadows.
}

// MARK: - Titles

- (id)currentTitle {
    let state: UIControlState = msg![env; this state];
    msg![env; this titleForState:state]
}
- (id)titleForState:(UIControlState)state {
    let host = env.objc.borrow::<UIButtonHostObject>(this);
    host.titles_for_states.get(&state)
        .or_else(|| host.titles_for_states.get(&UIControlStateNormal))
        .copied().unwrap_or(nil)
}
- (())setTitle:(id)title forState:(UIControlState)state {
    retain(env, title);
    let old = env.objc.borrow_mut::<UIButtonHostObject>(this)
        .titles_for_states.insert(state, title);
    release(env, old.unwrap_or(nil));
    update(env, this);
}

- (id)currentAttributedTitle { nil }
- (())setAttributedTitle:(id)_title forState:(UIControlState)_state {
    log!("UIButton setAttributedTitle:forState: stubbed");
}
- (id)attributedTitleForState:(UIControlState)_state { nil }

// MARK: - Title colors

- (id)currentTitleColor {
    let state: UIControlState = msg![env; this state];
    msg![env; this titleColorForState:state]
}
- (id)titleColorForState:(UIControlState)state {
    let host = env.objc.borrow::<UIButtonHostObject>(this);
    host.title_colors_for_states.get(&state)
        .or_else(|| host.title_colors_for_states.get(&UIControlStateNormal))
        .copied().unwrap_or(nil)
}
- (())setTitleColor:(id)color forState:(UIControlState)state {
    retain(env, color);
    let old = env.objc.borrow_mut::<UIButtonHostObject>(this)
        .title_colors_for_states.insert(state, color);
    release(env, old.unwrap_or(nil));
    update(env, this);
}

- (id)currentTitleShadowColor { nil }
- (())setTitleShadowColor:(id)_color forState:(UIControlState)_state {}
- (id)titleShadowColorForState:(UIControlState)_state { nil }

// MARK: - Images

- (id)currentImage {
    let state: UIControlState = msg![env; this state];
    msg![env; this imageForState:state]
}
- (id)imageForState:(UIControlState)state {
    let host = env.objc.borrow::<UIButtonHostObject>(this);
    host.images_for_states.get(&state)
        .or_else(|| host.images_for_states.get(&UIControlStateNormal))
        .copied().unwrap_or(nil)
}
- (())setImage:(id)image forState:(UIControlState)state {
    retain(env, image);
    let old = env.objc.borrow_mut::<UIButtonHostObject>(this)
        .images_for_states.insert(state, image);
    release(env, old.unwrap_or(nil));
    update(env, this);
}

- (id)currentBackgroundImage {
    let state: UIControlState = msg![env; this state];
    msg![env; this backgroundImageForState:state]
}
- (id)backgroundImageForState:(UIControlState)state {
    let host = env.objc.borrow::<UIButtonHostObject>(this);
    host.background_images_for_states.get(&state)
        .or_else(|| host.background_images_for_states.get(&UIControlStateNormal))
        .copied().unwrap_or(nil)
}
- (())setBackgroundImage:(id)image forState:(UIControlState)state {
    retain(env, image);
    let old = env.objc.borrow_mut::<UIButtonHostObject>(this)
        .background_images_for_states.insert(state, image);
    release(env, old.unwrap_or(nil));
    update(env, this);
}

// MARK: - Hit testing

- (id)hitTest:(CGPoint)point withEvent:(id)event {
    if msg![env; this pointInside:point withEvent:event] { this } else { nil }
}

@end

// MARK: - UIRoundedRectButton

@implementation UIRoundedRectButton: UIButton
@end

// MARK: - UIButtonContent

@implementation UIButtonContent: NSObject

+ (id)alloc {
    let host_object = Box::<UIButtonContentHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let title_key = get_static_str(env, "UITitle");
    let title: id = msg![env; coder decodeObjectForKey:title_key];
    log_dbg!("UIButtonContent UITitle -> {}", to_rust_string(env, title));

    let title_color_key = get_static_str(env, "UITitleColor");
    let title_color: id = msg![env; coder decodeObjectForKey:title_color_key];

    let image_key = get_static_str(env, "UIImage");
    let image: id = msg![env; coder decodeObjectForKey:image_key];
    log_dbg!("UIButtonContent initWithCoder: UIImage -> {:?}", image);

    let bg_image_key = get_static_str(env, "UIBackgroundImage");
    let background_image: id = msg![env; coder decodeObjectForKey:bg_image_key];
    log_dbg!("UIButtonContent initWithCoder: UIBackgroundImage -> {:?}", background_image);

    retain(env, title);
    retain(env, title_color);
    retain(env, image);
    retain(env, background_image);
    let host_obj = env.objc.borrow_mut::<UIButtonContentHostObject>(this);
    host_obj.title = title;
    host_obj.title_color = title_color;
    host_obj.image = image;
    host_obj.background_image = background_image;
    this
}

- (id)title       { env.objc.borrow::<UIButtonContentHostObject>(this).title }
- (id)titleColor  { env.objc.borrow::<UIButtonContentHostObject>(this).title_color }
- (id)image       { env.objc.borrow::<UIButtonContentHostObject>(this).image }
- (id)backgroundImage { env.objc.borrow::<UIButtonContentHostObject>(this).background_image }
- (id)shadowColor { nil }

- (id)description {
    let title = env.objc.borrow::<UIButtonContentHostObject>(this).title;
    let title_color = env.objc.borrow::<UIButtonContentHostObject>(this).title_color;
    let s = format!("UIButtonContent({this:?}, title {title:?}, color {title_color:?})");
    let ns_str = from_rust_string(env, s);
    autorelease(env, ns_str)
}

- (())dealloc {
    let &UIButtonContentHostObject { title, title_color, image, background_image } = env.objc.borrow(this);
    release(env, title);
    release(env, title_color);
    release(env, image);
    release(env, background_image);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
