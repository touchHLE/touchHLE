/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActionSheet`.

use crate::objc::{id, msg_super, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIActionSheet: UIView

- (id)initWithTitle:(id)title
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
       destructiveButtonTitle:(id)destructiveButtonTitle
            otherButtonTitles:(id)otherButtonTitles {

    log!("TODO: [(UIActionSheet*){:?} initWithTitle:{:?} delegate:{:?} cancelButtonTitle:{:?} destructiveButtonTitle:{:?} otherButtonTitles:{:?}]", this, title, delegate, cancelButtonTitle, destructiveButtonTitle, otherButtonTitles);

    msg_super![env; this init]
}

@end

};
