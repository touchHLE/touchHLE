/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITextField`.

use crate::frameworks::foundation::{ns_string, NSInteger};
use crate::objc::{id, objc_classes, ClassExports};

type UIKeyboardAppearance = NSInteger;
type UIKeyboardType = NSInteger;
type UIReturnKeyType = NSInteger;
type UITextAutocapitalizationType = NSInteger;
type UITextAutocorrectionType = NSInteger;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITextField: UIControl

// TODO: rendering
// TODO: more properties

- (id)text {
    // This should be `nil` by default, but Wolf3d crashes otherwise
    ns_string::get_static_str(env, "")
}

- (())setText:(id)_text { // NSString*
    // TODO
}
- (())setTextColor:(id)_color { // UIColor*
    // TODO: implement this once views are actually rendered
}

- (())setClearsOnBeginEditing:(bool)_clear {
    // TODO
}

// weak/non-retaining
- (())setDelegate:(id)_delegate { // something implementing UITextFieldDelegate
    // TODO
}

// UITextInputTraits implementation
- (())setAutocapitalizationType:(UITextAutocapitalizationType)_type {
    // TODO
}
- (())setAutocorrectionType:(UITextAutocorrectionType)_type {
    // TODO
}
- (())setReturnKeyType:(UIReturnKeyType)_type {
    // TODO
}
- (())setKeyboardAppearance:(UIKeyboardAppearance)_appearance {
    // TODO
}
- (())setKeyboardType:(UIKeyboardType)_type {
    // TODO
}

@end

};
