/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIControl`.

pub mod ui_button;
pub mod ui_text_field;

use crate::frameworks::foundation::NSUInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

struct UIControlHostObject {
    superclass: super::UIViewHostObject,
    enabled: bool,
    selected: bool,
    highlighted: bool,
    /// `UITouch*` of the touch currently being tracked, [nil] if none
    tracked_touch: id,
    tracking: bool,
}
impl_HostObject_with_superclass!(UIControlHostObject);
impl Default for UIControlHostObject {
    fn default() -> Self {
        UIControlHostObject {
            superclass: Default::default(),
            enabled: true,
            selected: false,
            highlighted: false,
            tracked_touch: nil,
            tracking: false,
        }
    }
}

type UIControlState = NSUInteger;
const UIControlStateNormal: UIControlState = 0;
const UIControlStateHighlighted: UIControlState = 1 << 0;
const UIControlStateDisabled: UIControlState = 1 << 1;
const UIControlStateSelected: UIControlState = 1 << 2;
#[allow(dead_code)]
const UIControlStateFocused: UIControlState = 1 << 3;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// abstract class
@implementation UIControl: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIControlHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let UIControlHostObject {
        superclass: _,
        enabled: _,
        selected: _,
        highlighted: _,
        tracking: _,
        tracked_touch,
    } = std::mem::take(env.objc.borrow_mut(this));

    release(env, tracked_touch);

    msg_super![env; this dealloc]
}

- (UIControlState)state {
    let &UIControlHostObject {
        superclass: _,
        tracking: _,
        tracked_touch: _,
        highlighted,
        enabled,
        selected,
    } = env.objc.borrow(this);
    // TODO: focussed
    let mut state = 0; // aka UIControlStateNormal
    if highlighted {
        state |= UIControlStateHighlighted;
    }
    if !enabled {
        state |= UIControlStateDisabled;
    }
    if selected {
        state |= UIControlStateSelected;
    }
    state
}

- (bool)isEnabled {
    env.objc.borrow::<UIControlHostObject>(this).enabled
}
- (())setEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIControlHostObject>(this).enabled = enabled;
}

- (bool)isSelected {
    env.objc.borrow::<UIControlHostObject>(this).selected
}
- (())setSelected:(bool)selected {
    env.objc.borrow_mut::<UIControlHostObject>(this).selected = selected;
}

- (bool)isHighlighted {
    env.objc.borrow::<UIControlHostObject>(this).highlighted
}
- (())setHighlighted:(bool)highlighted {
    env.objc.borrow_mut::<UIControlHostObject>(this).highlighted = highlighted;
}

- (bool)tracking {
    env.objc.borrow::<UIControlHostObject>(this).tracking
}

- (bool)beginTrackingWithTouch:(id)_touch // UITouch*
                     withEvent:(id)_event { // UIEvent*
    // default implementation, subclasses can override this
    true
}
- (bool)continueTrackingWithTouch:(id)_touch // UITouch*
                        withEvent:(id)_event { // UIEvent*
    // default implementation, subclasses can override this
    true
}
- (())endTrackingWithTouch:(id)_touch // UITouch*
                  withEvent:(id)_event { // UIEvent*
    // default implementation, subclasses can override this, must call super
    // (for some reason, the docs say this default implementation updates the
    // tracking property? why here?)
    env.objc.borrow_mut::<UIControlHostObject>(this).tracking = false;
}

- (())touchesBegan:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    if !msg![env; this isEnabled] {
        return;
    }

    // TODO: It seems like UIControl can also handle touch events that don't
    // begin within this control, but in fact elsewhere. How do we handle those?

    // UIControl's documentation implies that only one touch is ever tracked
    // at once.
    let touch: id = msg![env; touches anyObject];
    if !msg![env; this beginTrackingWithTouch:touch withEvent:event] {
        return;
    }

    retain(env, touch);
    let host_obj = env.objc.borrow_mut::<UIControlHostObject>(this);
    host_obj.tracking = true;
    let old_touch = std::mem::replace(&mut host_obj.tracked_touch, touch);
    release(env, old_touch);
    if old_touch != nil {
        log!("Got new touch {:?} but old touch {:?} has not yet ended!", touch, old_touch);
    }
    // Not sure if this is the right place to set this.
    () = msg![env; this setHighlighted:true];
}
- (())touchesMoved:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    if !msg![env; this isEnabled] {
        return;
    }

    let touch: id = msg![env; touches anyObject];
    let tracked_touch = env.objc.borrow::<UIControlHostObject>(this).tracked_touch;
    if tracked_touch != touch {
        return;
    }
    if !msg![env; this continueTrackingWithTouch:touch withEvent:event] {
        release(env, tracked_touch);
        env.objc.borrow_mut::<UIControlHostObject>(this).tracked_touch = nil;
        env.objc.borrow_mut::<UIControlHostObject>(this).tracking = false;
        () = msg![env; this setHighlighted:false];
    }
}
- (())touchesEnded:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    let touch: id = msg![env; touches anyObject];
    let tracked_touch = env.objc.borrow::<UIControlHostObject>(this).tracked_touch;
    if tracked_touch != touch {
        return;
    }
    () = msg![env; this endTrackingWithTouch:touch withEvent:event];
    release(env, tracked_touch);
    env.objc.borrow_mut::<UIControlHostObject>(this).tracked_touch = nil;
    () = msg![env; this setHighlighted:false];
}

// TODO: triggers/targets/actions

@end

};
