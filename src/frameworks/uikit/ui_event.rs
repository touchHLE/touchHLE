/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIEvent`.

use super::ui_touch::UITouchHostObject;
use crate::frameworks::core_graphics::CGPoint;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

pub(super) struct UIEventHostObject {
    /// `NSSet<UITouch*>*`
    touches: id,
    /// `UIView*`
    pub(super) view: id,
}
impl HostObject for UIEventHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIEvent: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIEventHostObject {
        touches: nil,
        view: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &UIEventHostObject { touches, view } = env.objc.borrow(this);
    release(env, touches);
    release(env, view);
}

- (id)touchesForView:(id)view {
    let &UIEventHostObject { touches, .. } = env.objc.borrow(this);
    // TODO: broken for multi-touch
    let touch: id = msg![env; touches anyObject];
    let &UITouchHostObject { original_location, window, .. } = env.objc.borrow(touch);
    // FIXME: handle non-zero-origin windows
    let location_in_view: CGPoint = msg![env; window convertPoint:original_location toView:view];
    if msg![env; view pointInside:location_in_view withEvent:this] {
        msg_class![env; NSSet setWithObject:touch]
    } else {
        let empty_set: id = msg_class![env; NSSet new];
        autorelease(env, empty_set)
    }
}

- (id)allTouches {
    let &UIEventHostObject { touches, .. } = env.objc.borrow(this);
    touches
}

// TODO: more accessors

@end

};

/// For use by [super::ui_touch]: create a `UIEvent` with a set of `UITouch*`
/// and the view it was originally sent to.
pub(super) fn new_event(env: &mut Environment, touches: id, view: id) -> id {
    let event: id = msg_class![env; UIEvent alloc];
    retain(env, touches);
    retain(env, view);
    let borrow = env.objc.borrow_mut::<UIEventHostObject>(event);
    borrow.touches = touches;
    borrow.view = view;
    event
}
