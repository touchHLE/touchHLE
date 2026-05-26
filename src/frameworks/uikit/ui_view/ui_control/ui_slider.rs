/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.

use super::UIControlState;
use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg_super, objc_classes, todo_objc_setter, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISlider*){:?} initWithFrame:{:?}] TODO: Implement UISlider. The control won't be rendered.", this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISlider*){:?} initWithCoder:{:?}] TODO: Implement UISlider. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

- (())setMinimumValueImage:(id)img { // UIImage *
    todo_objc_setter!(this, img);
}
- (())setMaximumValueImage:(id)img { // UIImage *
    todo_objc_setter!(this, img);
}

- (())setMinimumValue:(f32)value {
    todo_objc_setter!(this, value);
}

- (())setMaximumValue:(f32)value {
    todo_objc_setter!(this, value);
}

- (())setValue:(f32)value {
    todo_objc_setter!(this, value);
}

- (())setValue:(f32)value animated:(bool)animated {
    log!("TODO: [(UISlider*){:?} setValue:{} animated:{}]", this, value, animated);
}

- (())setThumbImage:(id)img
          forState:(UIControlState)state {
    log!("TODO: [(UISlider*){:?} setThumbImage:{:?} forState:{}]", this, img, state);
}

- (())setMinimumTrackImage:(id)img
                 forState:(UIControlState)state {
    log!("TODO: [(UISlider*){:?} setMinimumTrackImage:{:?} forState:{}]", this, img, state);
}

- (())setMaximumTrackImage:(id)img
                 forState:(UIControlState)state {
    log!("TODO: [(UISlider*){:?} setMaximumTrackImage:{:?} forState:{}]", this, img, state);
}

// TODO: all of it

@end

};
