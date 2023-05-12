/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIImageView`.

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::objc::{id, msg, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImageView: UIView
// TODO: actually display images etc. Currently touchHLE can't draw views, so
// there's no point implementing that yet.

// initWithCoder: intentionally not overridden yet

- (id)initWithImage:(id)image { // UIImage*
    let size: CGSize = msg![env; image size];
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size
    };
    msg![env; this initWithFrame:frame]
}

- (())setImage:(id)_image { // UIImage*
    // TODO: implement
}

@end

};
