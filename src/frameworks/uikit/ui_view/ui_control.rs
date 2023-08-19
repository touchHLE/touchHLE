/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIControl`.

pub mod ui_button;
pub mod ui_text_field;

use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, impl_HostObject_with_superclass, objc_classes, ClassExports, NSZonePtr};

struct UIControlHostObject {
    superclass: super::UIViewHostObject,
    enabled: bool,
    selected: bool,
    highlighted: bool,
}
impl_HostObject_with_superclass!(UIControlHostObject);
impl Default for UIControlHostObject {
    fn default() -> Self {
        UIControlHostObject {
            superclass: Default::default(),
            enabled: true,
            selected: false,
            highlighted: false,
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

// TODO: state, triggers, etc

- (UIControlState)state {
    let &UIControlHostObject {
        superclass: _,
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

@end

};
