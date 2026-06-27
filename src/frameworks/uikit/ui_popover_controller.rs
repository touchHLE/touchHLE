/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPopoverController` (iPad only — stub implementation).

use crate::frameworks::core_graphics::{CGRect, CGSize};
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// UIPopoverArrowDirection bitmask constants
pub type UIPopoverArrowDirection = NSUInteger;
pub const UIPopoverArrowDirectionUp: UIPopoverArrowDirection = 1 << 0;
pub const UIPopoverArrowDirectionDown: UIPopoverArrowDirection = 1 << 1;
pub const UIPopoverArrowDirectionLeft: UIPopoverArrowDirection = 1 << 2;
pub const UIPopoverArrowDirectionRight: UIPopoverArrowDirection = 1 << 3;
pub const UIPopoverArrowDirectionAny: UIPopoverArrowDirection = UIPopoverArrowDirectionUp
    | UIPopoverArrowDirectionDown
    | UIPopoverArrowDirectionLeft
    | UIPopoverArrowDirectionRight;
pub const UIPopoverArrowDirectionUnknown: UIPopoverArrowDirection = NSUInteger::MAX;

#[derive(Default)]
struct UIPopoverControllerHostObject {
    /// The content view controller displayed inside the popover.
    content_view_controller: id,
    /// Weak delegate reference.
    delegate: id,
    /// Whether the popover is currently visible.
    popover_visible: bool,
    /// The popover's content size.
    popover_content_size: CGSize,
    /// Passthrough views — NSArray* of UIView*.
    passthrough_views: id,
    /// Background color — UIColor*.
    background_color: id,
    /// Arrow direction actually used (set after presentation).
    popover_arrow_direction: UIPopoverArrowDirection,
}
impl HostObject for UIPopoverControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPopoverController: NSObject

// =========================================================================
// MARK: - Allocation
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPopoverControllerHostObject {
        content_view_controller: nil,
        delegate: nil,
        popover_visible: false,
        popover_content_size: CGSize { width: 320.0, height: 480.0 },
        passthrough_views: nil,
        background_color: nil,
        popover_arrow_direction: UIPopoverArrowDirectionUnknown,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// =========================================================================
// MARK: - Class availability
// =========================================================================

+ (bool)instancesRespondToSelector:(id)_selector {
    log_dbg!("UIPopoverController instancesRespondToSelector: => YES");
    true
}

+ (bool)respondsToSelector:(id)_selector {
    log_dbg!("UIPopoverController respondsToSelector: => YES");
    true
}

// =========================================================================
// MARK: - Initializers
// =========================================================================

- (id)init {
    this
}

- (id)initWithContentViewController:(id)view_controller {
    retain(env, view_controller);
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
        .content_view_controller = view_controller;

    // Use the VC's preferredContentSize as the initial popover size if set.
    if view_controller != nil {
        let preferred: CGSize = msg![env; view_controller preferredContentSize];
        if preferred.width > 0.0 && preferred.height > 0.0 {
            env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
                .popover_content_size = preferred;
        }
    }
    this
}

// =========================================================================
// MARK: - Dealloc
// =========================================================================

- (())dealloc {
    let host = env.objc.borrow::<UIPopoverControllerHostObject>(this);
    let (vc, delegate, passthrough, bg) = (
        host.content_view_controller,
        host.delegate,
        host.passthrough_views,
        host.background_color,
    );
    release(env, vc);
    release(env, delegate);
    release(env, passthrough);
    release(env, bg);
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// MARK: - Content view controller
// =========================================================================

- (id)contentViewController {
    env.objc.borrow::<UIPopoverControllerHostObject>(this).content_view_controller
}

- (())setContentViewController:(id)view_controller {
    let _: () = msg![env; this setContentViewController:view_controller animated:false];
}

- (())setContentViewController:(id)view_controller animated:(bool)_animated {
    let old = env.objc.borrow::<UIPopoverControllerHostObject>(this)
        .content_view_controller;
    release(env, old);
    retain(env, view_controller);
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
        .content_view_controller = view_controller;
}

// =========================================================================
// MARK: - Content size
// =========================================================================

- (CGSize)popoverContentSize {
    env.objc.borrow::<UIPopoverControllerHostObject>(this).popover_content_size
}

- (())setPopoverContentSize:(CGSize)size {
    let _: () = msg![env; this setPopoverContentSize:size animated:false];
}

- (())setPopoverContentSize:(CGSize)size animated:(bool)_animated {
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
        .popover_content_size = size;
}

// =========================================================================
// MARK: - Delegate
// =========================================================================

- (id)delegate {
    env.objc.borrow::<UIPopoverControllerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<UIPopoverControllerHostObject>(this).delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this).delegate = delegate;
}

// =========================================================================
// MARK: - Passthrough views
// =========================================================================

- (id)passthroughViews { // NSArray*
    env.objc.borrow::<UIPopoverControllerHostObject>(this).passthrough_views
}

- (())setPassthroughViews:(id)views { // NSArray*
    let old = env.objc.borrow::<UIPopoverControllerHostObject>(this).passthrough_views;
    release(env, old);
    retain(env, views);
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this).passthrough_views = views;
}

// =========================================================================
// MARK: - Background color (iOS 7+)
// =========================================================================

- (id)backgroundColor { // UIColor*
    env.objc.borrow::<UIPopoverControllerHostObject>(this).background_color
}

- (())setBackgroundColor:(id)color { // UIColor*
    let old = env.objc.borrow::<UIPopoverControllerHostObject>(this).background_color;
    release(env, old);
    retain(env, color);
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this).background_color = color;
}

// =========================================================================
// MARK: - Visibility / arrow direction
// =========================================================================

- (bool)isPopoverVisible {
    env.objc.borrow::<UIPopoverControllerHostObject>(this).popover_visible
}

- (UIPopoverArrowDirection)popoverArrowDirection {
    env.objc.borrow::<UIPopoverControllerHostObject>(this).popover_arrow_direction
}

// =========================================================================
// MARK: - Presentation
// touchHLE has no popover UI.
// We immediately fire the delegate's
// popoverControllerDidDismissPopover: so the app can clean up, matching
// the pattern used for other picker/sheet controllers.
// =========================================================================

- (())presentPopoverFromRect:(CGRect)rect
                      inView:(id)_view
    permittedArrowDirections:(UIPopoverArrowDirection)arrow_directions
                    animated:(bool)_animated {
    log_dbg!(
        "UIPopoverController presentPopoverFromRect:{:?} arrowDirections:{:#x} — dismissing immediately",
        rect, arrow_directions
    );
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this).popover_visible = true;
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
        .popover_arrow_direction = UIPopoverArrowDirectionUnknown;

    let _: () = msg![env; this dismissPopoverAnimated:false];
}

- (())presentPopoverFromBarButtonItem:(id)_item
             permittedArrowDirections:(UIPopoverArrowDirection)arrow_directions
                             animated:(bool)_animated {
    log_dbg!(
        "UIPopoverController presentPopoverFromBarButtonItem: arrowDirections:{:#x} — dismissing immediately",
        arrow_directions
    );
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this).popover_visible = true;
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
        .popover_arrow_direction = UIPopoverArrowDirectionUnknown;

    let _: () = msg![env; this dismissPopoverAnimated:false];
}

// =========================================================================
// MARK: - Dismissal
// =========================================================================

- (())dismissPopoverAnimated:(bool)_animated {
    if !env.objc.borrow::<UIPopoverControllerHostObject>(this).popover_visible {
        return;
    }
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this).popover_visible = false;
    env.objc.borrow_mut::<UIPopoverControllerHostObject>(this)
        .popover_arrow_direction = UIPopoverArrowDirectionUnknown;

    let delegate = env.objc.borrow::<UIPopoverControllerHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc
            .lookup_selector("popoverControllerDidDismissPopover:")
            .unwrap();
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate popoverControllerDidDismissPopover:this];
        }
    }
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let (visible, size) = {
        let h = env.objc.borrow::<UIPopoverControllerHostObject>(this);
        (h.popover_visible, h.popover_content_size)
    };

    // Выносим поля из упакованной структуры CGSize в локальные переменные
    let width = size.width;
    let height = size.height;

    let s = format!(
        "<UIPopoverController: {:?}; visible={}; contentSize={{{}, {}}}>",
        this, visible, width, height
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};
