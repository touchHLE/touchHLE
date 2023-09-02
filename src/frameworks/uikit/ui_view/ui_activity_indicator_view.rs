/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActivityIndicatorView`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg_super, objc_classes, ClassExports};

#[allow(dead_code)]
pub enum UIActivityIndicatorViewStyle {
    UIActivityIndicatorViewStyleWhiteLarge = 0,
    UIActivityIndicatorViewStyleWhite = 1,
    UIActivityIndicatorViewStyleGray = 2,
    UIActivityIndicatorViewStyleMedium = 100,
    UIActivityIndicatorViewStyleLarge = 101,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIActivityIndicatorView: UIView

- (id)initWithActivityIndicatorStyle:(NSInteger)style {
    log!("TODO: [(UIActivityIndicatorView*){:?} initWithActivityIndicatorStyle:{:?}]", this, style);
    msg_super![env; this init]
}

- (())startAnimating {
    log!("TODO: [(UIActivityIndicatorView*){:?} startAnimating", this);
}

- (())stopAnimating {
    log!("TODO: [(UIActivityIndicatorView*){:?} stopAnimating", this);
}

@end

};
