/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIImageView`.

use super::{UIViewHostObject, UIViewSubclass};
use crate::frameworks::core_graphics::cg_image::CGImageRef;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::objc::{id, msg, msg_super, nil, objc_classes, release, retain, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImageView: UIView

- (id)init {
    let this: id = msg_super![env; this init];
    // Not sure if UIImageView does this unconditionally, or only for images
    // with alpha channels.
    () = msg![env; this setOpaque:false];
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    host_obj.subclass = UIViewSubclass::UIImageView {
        image: nil
    };
    this
}

- (())dealloc {
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    let subclass = std::mem::take(&mut host_obj.subclass);
    let UIViewSubclass::UIImageView { image } = subclass else {
        panic!();
    };
    release(env, image);
    msg_super![env; this dealloc]
}

// TODO: initWithCoder:

- (id)initWithImage:(id)image { // UIImage*
    let size: CGSize = msg![env; image size];
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size
    };
    let this = msg![env; this initWithFrame:frame];
    () = msg![env; this setImage:image];
    this
}

- (id)image {
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    let &UIViewSubclass::UIImageView { image } = &host_obj.subclass else {
        panic!();
    };
    image
}

- (())setImage:(id)new_image { // UIImage*
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
    let UIViewSubclass::UIImageView { ref mut image } = host_obj.subclass else {
        panic!();
    };
    let old_image = std::mem::replace(image, new_image);
    retain(env, new_image);
    release(env, old_image);

    let layer: id = msg![env; this layer];
    () = msg![env; layer setNeedsDisplay];
}

// Normally a UIKit view is drawn into a CGContextRef by drawRect:, which is
// presumably called from drawLayer:inContext:. But for UIImageView, this would
// be wasteful, we can tell Core Animation to display the image directly rather
// than copying it to a (CGBitmapContext). If displayLayer: is defined, then
// drawLayer:inContext: doesn't get called, so I assume this is what the real
// UIKit does?
- (())displayLayer:(id)layer {
    let image: id = msg![env; this image];
    let cg_image: CGImageRef = msg![env; image CGImage];
    () = msg![env; layer setContents:cg_image];
}

@end

};
