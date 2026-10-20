/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISegmentedControl`.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg_super, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISegmentedControl: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISegmentedControl*){:?} initWithFrame:{:?}] TODO: Implement UISegmentedControl. The control won't be rendered.", this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISegmentedControl*){:?} initWithCoder:{:?}] TODO: Implement UISegmentedControl. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

// TODO: all of it

@end

// Undocumented class used by UISegmentedControl
@implementation UISegment: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISegment*){:?} initWithFrame:{:?}] Attempted to initialize undocumented class from outside of the NIB.", this, frame);
    unreachable!()
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISegment*){:?} initWithCoder:{:?}] TODO: Implement UISegment. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

// TODO: all of it

@end

};
