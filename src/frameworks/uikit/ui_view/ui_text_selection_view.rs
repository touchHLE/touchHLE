/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITextSelectionView`.
//!
//! This is a private UIKit class used internally by UITextField and UITextView
//! to display text selection handles and highlight regions. It is a subclass
//! of UIView.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, objc_classes, ClassExports, NSZonePtr,
};

#[derive(Default)]
struct UITextSelectionViewHostObject {
    superclass: super::UIViewHostObject,
    /// The UIView that this selection view is tracking selections for.
    /// Weak reference (non-retaining).
    target_view: id,
}
impl_HostObject_with_superclass!(UITextSelectionViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITextSelectionView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITextSelectionViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    () = msg![env; this setOpaque:false];
    () = msg![env; this setUserInteractionEnabled:false];
    () = msg![env; this setHidden:true];
    this
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    () = msg![env; this setOpaque:false];
    () = msg![env; this setUserInteractionEnabled:false];
    () = msg![env; this setHidden:true];
    this
}

- (())dealloc {
    msg_super![env; this dealloc]
}

- (())setTextSelectionView:(id)view {
    env.objc.borrow_mut::<UITextSelectionViewHostObject>(this).target_view = view;
}

- (id)textSelectionView {
    env.objc.borrow::<UITextSelectionViewHostObject>(this).target_view
}

@end

};
