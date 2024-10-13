/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActivityIndicatorView`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg, ClassExports};
use crate::objc_classes;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIActivityIndicatorView: UIView

- (id)initWithActivityIndicatorStyle:(NSInteger)_style {
    // TODO: proper init
    msg![env; this init]
}

- (())startAnimating {
    log!("TODO: [(UIActivityIndicatorView *){:?} startAnimating]", this);
}
- (())stopAnimating {
    log!("TODO: [(UIActivityIndicatorView *){:?} stopAnimating]", this);
}

@end

};
