/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIEvent`.

use crate::frameworks::core_graphics::CGPoint;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;

struct UIEventHostObject {
    /// `NSSet<UITouch*>*`
    touches: id,
    /// `UIView*`
    view: id,
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
    // FIXME: this will be wrong sometimes. locationInView: currently panics
    // if it would be, at least.
    let touch: id = msg![env; touches anyObject];
    let _: CGPoint = msg![env; touch locationInView:view];
    touches
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
