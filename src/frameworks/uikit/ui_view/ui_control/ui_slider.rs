/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg_super, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("TODO: Implement UISlider. The control won't be rendered.");
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("TODO: Implement UISlider. The control won't be rendered.");
    msg_super![env; this initWithCoder:coder]
}


// TODO: all of it

@end

};
