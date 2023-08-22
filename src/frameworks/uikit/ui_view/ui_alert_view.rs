/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg_super, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertView: UIView
- (id)initWithTitle:(id)title
                      message:(id)message
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
            otherButtonTitles:(id)otherButtonTitles {

    log!("TODO: [(UIAlertView*){:?} initWithTitle:{:?} message:{:?} delegate:{:?} cancelButtonTitle:{:?} otherButtonTitles:{:?}]", this, title, message, delegate, cancelButtonTitle, otherButtonTitles);

    let msg = ns_string::to_rust_string(env, message);
    let title = ns_string::to_rust_string(env, title);
    log!("UIAlertView: title: {:?}, message: {:?}", title, msg);

    msg_super![env; this init]
}
- (())show {
    log!("TODO: [(UIAlertView*){:?} show]", this);
}
@end

};
