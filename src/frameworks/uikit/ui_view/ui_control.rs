/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIControl`.

pub mod ui_button;
pub mod ui_text_field;

use super::{UIViewHostObject, UIViewSubclass};
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, msg_super, objc_classes, ClassExports};

pub struct UIControlData {
    subclass: UIControlSubclass,
}

#[derive(Default)]
pub(super) enum UIControlSubclass {
    #[default]
    /// Subclass that doesn't need extra data.
    UIControl,
    UIButton(ui_button::UIButtonData),
}

type UIControlState = NSUInteger;
const UIControlStateNormal: UIControlState = 0;
#[allow(dead_code)]
const UIControlStateHighlighted: UIControlState = 1 << 0;
#[allow(dead_code)]
const UIControlStateDisabled: UIControlState = 1 << 1;
#[allow(dead_code)]
const UIControlStateSelected: UIControlState = 1 << 2;
#[allow(dead_code)]
const UIControlStateFocused: UIControlState = 1 << 3;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// abstract class
@implementation UIControl: UIView

- (id)init {
    let this: id = msg_super![env; this init];
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    host_obj.subclass = UIViewSubclass::UIControl(UIControlData {
        subclass: UIControlSubclass::UIControl,
    });
    this
}

- (())dealloc {
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    let subclass = std::mem::take(&mut host_obj.subclass);
    let UIViewSubclass::UIControl(data) = subclass else {
        panic!();
    };
    // This assert forces subclasses to clean up their data in their dealloc
    // implementation :)
    assert!(matches!(data.subclass, UIControlSubclass::UIControl));
    msg_super![env; this dealloc]
}

// TODO: state, triggers, etc

- (())setEnabled:(bool)_enabled {
    // TODO
}

@end

};
