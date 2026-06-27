/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINavigationBar`.

use crate::frameworks::core_graphics::{CGRect, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_view::UIViewHostObject;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, HostObject, NSZonePtr,
};

type UIBarStyle = NSInteger;
const UIBarStyleDefault: UIBarStyle = 0;
const UIBarStyleBlack: UIBarStyle = 1;
const UIBarStyleBlackOpaque: UIBarStyle = 2;
const UIBarStyleBlackTranslucent: UIBarStyle = 3;

#[derive(Default)]
struct UINavigationBarHostObject {
    /// Embedded `UIView` host state. `UINavigationBar` is a `UIView` subclass,
    /// so it must carry the superclass host object; without it every `UIView`
    /// method (frame, subviews, layout, ...) sent to a nav bar fails to borrow
    /// and falls back to a zeroed phantom.
    superclass: UIViewHostObject,
    /// UINavigationBarDelegate — weak reference
    delegate: id,
    bar_style: UIBarStyle,
    translucent: bool,
    tint_color: id,            // UIColor* — retained
    bar_tint_color: id,        // UIColor* — retained
    title_text_attributes: id, // NSDictionary* — retained
    /// NSMutableArray* of UINavigationItem* — retained
    items: id,
    /// UIImage* — retained
    shadow_image: id,
    /// UIImage* background — retained
    background_image: id,
}
impl_HostObject_with_superclass!(UINavigationBarHostObject);

// MARK: - UINavigationItem

#[derive(Default)]
struct UINavigationItemHostObject {
    title: id,        // NSString* — retained
    title_view: id,   // UIView* — retained
    prompt: id,       // NSString* — retained
    back_button: id,  // UIBarButtonItem* — retained
    left_button: id,  // UIBarButtonItem* — retained
    right_button: id, // UIBarButtonItem* — retained
    left_items: id,   // NSArray* — retained
    right_items: id,  // NSArray* — retained
    hides_back_button: bool,
    left_items_supplemented: bool,
}
impl HostObject for UINavigationItemHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationBar: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let items = msg_class![env; NSMutableArray new];
    let host_object = Box::new(UINavigationBarHostObject {
        superclass: Default::default(),
        delegate: nil,
        bar_style: UIBarStyleDefault,
        translucent: true,
        tint_color: nil,
        bar_tint_color: nil,
        title_text_attributes: nil,
        items,
        shadow_image: nil,
        background_image: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    // Run UIView's initializer so the backing layer and view state are set up.
    msg_super![env; this init]
}

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

- (id)initWithCoder:(id)coder {
    msg_super![env; this initWithCoder:coder]
}

- (())dealloc {
    let host = env.objc.borrow::<UINavigationBarHostObject>(this);
    let (tint_color, bar_tint_color, title_attrs, items, shadow, bg) = (
        host.tint_color,
        host.bar_tint_color,
        host.title_text_attributes,
        host.items,
        host.shadow_image,
        host.background_image,
    );
    release(env, tint_color);
    release(env, bar_tint_color);
    release(env, title_attrs);
    release(env, items);
    release(env, shadow);
    release(env, bg);
    // delegate is weak — no release
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Delegate

- (id)delegate {
    env.objc.borrow::<UINavigationBarHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    // Weak reference — no retain.
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).delegate = delegate;
}

// MARK: - Style

- (UIBarStyle)barStyle {
    env.objc.borrow::<UINavigationBarHostObject>(this).bar_style
}

- (())setBarStyle:(UIBarStyle)style {
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).bar_style = style;
}

- (bool)isTranslucent {
    env.objc.borrow::<UINavigationBarHostObject>(this).translucent
}

- (())setTranslucent:(bool)value {
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).translucent = value;
}

// MARK: - Colors

- (id)tintColor {
    env.objc.borrow::<UINavigationBarHostObject>(this).tint_color
}

- (())setTintColor:(id)color {
    let old = env.objc.borrow::<UINavigationBarHostObject>(this).tint_color;
    release(env, old);
    retain(env, color);
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).tint_color = color;
}

- (id)barTintColor {
    env.objc.borrow::<UINavigationBarHostObject>(this).bar_tint_color
}

- (())setBarTintColor:(id)color {
    let old = env.objc.borrow::<UINavigationBarHostObject>(this).bar_tint_color;
    release(env, old);
    retain(env, color);
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).bar_tint_color = color;
}

// MARK: - Title attributes

- (id)titleTextAttributes {
    env.objc.borrow::<UINavigationBarHostObject>(this).title_text_attributes
}

- (())setTitleTextAttributes:(id)attrs {
    let old = env.objc.borrow::<UINavigationBarHostObject>(this).title_text_attributes;
    release(env, old);
    retain(env, attrs);
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).title_text_attributes = attrs;
}

// MARK: - Background / shadow images

- (id)shadowImage {
    env.objc.borrow::<UINavigationBarHostObject>(this).shadow_image
}

- (())setShadowImage:(id)image {
    let old = env.objc.borrow::<UINavigationBarHostObject>(this).shadow_image;
    release(env, old);
    retain(env, image);
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).shadow_image = image;
}

- (id)backgroundImageForBarMetrics:(NSInteger)_metrics {
    env.objc.borrow::<UINavigationBarHostObject>(this).background_image
}

- (())setBackgroundImage:(id)image forBarMetrics:(NSInteger)_metrics {
    let old = env.objc.borrow::<UINavigationBarHostObject>(this).background_image;
    release(env, old);
    retain(env, image);
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).background_image = image;
}

- (())setBackgroundImage:(id)image
         forBarPosition:(NSInteger)_position
             barMetrics:(NSInteger)_metrics {
    () = msg![env; this setBackgroundImage:image forBarMetrics:_metrics];
}

// MARK: - Items stack

- (id)items {
    env.objc.borrow::<UINavigationBarHostObject>(this).items
}

- (())setItems:(id)items { // NSArray*
    let old_items = env.objc.borrow::<UINavigationBarHostObject>(this).items;
    release(env, old_items);
    let new_items: id = msg_class![env; NSMutableArray alloc];
    let new_items: id = msg![env; new_items initWithArray:items];
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).items = new_items;
}

- (())setItems:(id)items animated:(bool)_animated {
    () = msg![env; this setItems:items];
}

- (id)topItem {
    let items = env.objc.borrow::<UINavigationBarHostObject>(this).items;
    let count: u32 = msg![env; items count];
    if count == 0 { return nil; }
    msg![env; items objectAtIndex:(count - 1)]
}

- (id)backItem {
    let items = env.objc.borrow::<UINavigationBarHostObject>(this).items;
    let count: u32 = msg![env; items count];
    if count < 2 { return nil; }
    msg![env; items objectAtIndex:(count - 2)]
}

- (())pushNavigationItem:(id)item animated:(bool)_animated {
    let delegate = env.objc.borrow::<UINavigationBarHostObject>(this).delegate;
    // Ask delegate if we should push.
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "navigationBar:shouldPushItem:".to_string(), &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let should: bool = msg![env; delegate navigationBar:this shouldPushItem:item];
            if !should { return; }
        }
    }
    let items = env.objc.borrow::<UINavigationBarHostObject>(this).items;
    () = msg![env; items addObject:item];

    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "navigationBar:didPushItem:".to_string(), &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate navigationBar:this didPushItem:item];
        }
    }
}

- (id)popNavigationItemAnimated:(bool)_animated {
    let items = env.objc.borrow::<UINavigationBarHostObject>(this).items;
    let count: u32 = msg![env; items count];
    if count == 0 { return nil; }

    let top: id = msg![env; items objectAtIndex:(count - 1)];

    let delegate = env.objc.borrow::<UINavigationBarHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "navigationBar:shouldPopItem:".to_string(), &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let should: bool = msg![env; delegate navigationBar:this shouldPopItem:top];
            if !should { return nil; }
        }
    }

    () = msg![env; items removeLastObject];

    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "navigationBar:didPopItem:".to_string(), &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate navigationBar:this didPopItem:top];
        }
    }
    top
}

// MARK: - Size

- (CGSize)sizeThatFits:(CGSize)_size {
    // Standard navigation bar height.
    CGSize { width: 320.0, height: 44.0 }
}

@end

@implementation UINavigationItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UINavigationItemHostObject {
        title: nil,
        title_view: nil,
        prompt: nil,
        back_button: nil,
        left_button: nil,
        right_button: nil,
        left_items: nil,
        right_items: nil,
        hides_back_button: false,
        left_items_supplemented: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTitle:(id)title {
    retain(env, title);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).title = title;
    this
}

- (id)init {
    this
}

// --- ДОБАВЛЕНО: Поддержка загрузки из NIB-файла ---
- (id)initWithCoder:(id)_coder {
    msg_super![env; this init]
}
// --------------------------------------------------

- (())dealloc {
    let host = env.objc.borrow::<UINavigationItemHostObject>(this);
    let (title, title_view, prompt, back, left, right, left_items, right_items) = (
        host.title, host.title_view, host.prompt,
        host.back_button, host.left_button, host.right_button,
        host.left_items, host.right_items,
    );
    release(env, title);
    release(env, title_view);
    release(env, prompt);
    release(env, back);
    release(env, left);
    release(env, right);
    release(env, left_items);
    release(env, right_items);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)title {
    env.objc.borrow::<UINavigationItemHostObject>(this).title
}

- (())setTitle:(id)title {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).title;
    release(env, old);
    retain(env, title);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).title = title;
}

- (id)titleView {
    env.objc.borrow::<UINavigationItemHostObject>(this).title_view
}

- (())setTitleView:(id)view {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).title_view;
    release(env, old);
    retain(env, view);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).title_view = view;
}

- (id)prompt {
    env.objc.borrow::<UINavigationItemHostObject>(this).prompt
}

- (())setPrompt:(id)prompt {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).prompt;
    release(env, old);
    retain(env, prompt);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).prompt = prompt;
}

- (id)backBarButtonItem {
    env.objc.borrow::<UINavigationItemHostObject>(this).back_button
}

- (())setBackBarButtonItem:(id)item {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).back_button;
    release(env, old);
    retain(env, item);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).back_button = item;
}

- (id)leftBarButtonItem {
    env.objc.borrow::<UINavigationItemHostObject>(this).left_button
}

- (())setLeftBarButtonItem:(id)item {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).left_button;
    release(env, old);
    retain(env, item);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).left_button = item;
}

- (())setLeftBarButtonItem:(id)item animated:(bool)_animated {
    () = msg![env; this setLeftBarButtonItem:item];
}

- (id)rightBarButtonItem {
    env.objc.borrow::<UINavigationItemHostObject>(this).right_button
}

- (())setRightBarButtonItem:(id)item {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).right_button;
    release(env, old);
    retain(env, item);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).right_button = item;
}

- (())setRightBarButtonItem:(id)item animated:(bool)_animated {
    () = msg![env; this setRightBarButtonItem:item];
}

- (id)leftBarButtonItems {
    env.objc.borrow::<UINavigationItemHostObject>(this).left_items
}

- (())setLeftBarButtonItems:(id)items {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).left_items;
    release(env, old);
    retain(env, items);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).left_items = items;
    // Sync single-item accessor to first element.
    if items != nil {
        let count: u32 = msg![env; items count];
        let first: id = if count > 0 { msg![env; items objectAtIndex:0u32] } else { nil };
        () = msg![env; this setLeftBarButtonItem:first];
    }
}

- (id)rightBarButtonItems {
    env.objc.borrow::<UINavigationItemHostObject>(this).right_items
}

- (())setRightBarButtonItems:(id)items {
    let old = env.objc.borrow::<UINavigationItemHostObject>(this).right_items;
    release(env, old);
    retain(env, items);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).right_items = items;
    if items != nil {
        let count: u32 = msg![env; items count];
        let first: id = if count > 0 { msg![env; items objectAtIndex:0u32] } else { nil };
        () = msg![env; this setRightBarButtonItem:first];
    }
}

- (bool)hidesBackButton {
    env.objc.borrow::<UINavigationItemHostObject>(this).hides_back_button
}

- (())setHidesBackButton:(bool)value {
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).hides_back_button = value;
}

- (())setHidesBackButton:(bool)value animated:(bool)_animated {
    () = msg![env; this setHidesBackButton:value];
}

- (bool)leftItemsSupplementBackButton {
    env.objc.borrow::<UINavigationItemHostObject>(this).left_items_supplemented
}

- (())setLeftItemsSupplementBackButton:(bool)value {
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).left_items_supplemented = value;
}

@end

};
