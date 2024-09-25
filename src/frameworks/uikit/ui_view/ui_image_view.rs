/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIImageView`.

use crate::frameworks::core_graphics::cg_image::CGImageRef;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::NSTimeInterval;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

#[derive(Default)]
struct UIImageViewHostObject {
    superclass: super::UIViewHostObject,
    /// `UIImage*`
    image: id,
}
impl_HostObject_with_superclass!(UIImageViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImageView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIImageViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    // Not sure if UIImageView does this unconditionally, or only for images
    // with alpha channels.
    () = msg![env; this setOpaque:false];
    this
}

- (())dealloc {
    let &UIImageViewHostObject {
        superclass: _,
        image,
    } = env.objc.borrow(this);
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
    let this = msg_super![env; this initWithFrame:frame];
    () = msg![env; this setImage:image];
    // Not sure if UIImageView does this unconditionally, or only for images
    // with alpha channels.
    () = msg![env; this setOpaque:false];
    this
}

- (id)image {
    env.objc.borrow::<UIImageViewHostObject>(this).image
}

- (())setImage:(id)new_image { // UIImage*
    let host_obj = env.objc.borrow_mut::<UIImageViewHostObject>(this);
    let old_image = std::mem::replace(&mut host_obj.image, new_image);
    retain(env, new_image);
    release(env, old_image);

    let layer: id = msg![env; this layer];
    () = msg![env; layer setNeedsDisplay];
}

- (())setAnimationImages:(id)images { // NSArray<UIImage *>*
    log!("TODO: [(UIImageView*) {:?} setAnimationImages:{:?}]", this, images);
    // TODO: Use all images in the array instead of just the first one
    let first_image: id = msg![env; images objectAtIndex:0u32];
    () = msg![env; this setImage:first_image];
}

- (())setAnimationDuration:(NSTimeInterval)duration { // NSArray<UIImage *>*
    log!("TODO: [(UIImageView*) {:?} setAnimationDuration:{}]", this, duration);
}

- (())startAnimating {
    log!("TODO: [(UIImageView*) {:?} startAnimating]", this);
}

- (())stopAnimating {
    log!("TODO: [(UIImageView*) {:?} stopAnimating]", this);
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
