/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIControl`.

pub mod ui_button;
pub mod ui_text_field;

use crate::frameworks::foundation::NSUInteger;
use crate::objc::{objc_classes, ClassExports};

type UIControlHostObject = super::UIViewHostObject;

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

// TODO: state, triggers, etc

- (())setEnabled:(bool)_enabled {
    // TODO
}

@end

};
