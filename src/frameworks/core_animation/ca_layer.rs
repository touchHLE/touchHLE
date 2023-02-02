/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CALayer`.

use crate::objc::{id, msg, nil, objc_classes, release, ClassExports, HostObject};

pub(super) struct CALayerHostObject {
    /// Possibly nil, usually a UIView. This is a weak reference.
    delegate: id,
    opaque: bool,
    /// For CAEAGLLayer only
    pub(super) drawable_properties: id,
}
impl HostObject for CALayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CALayer: NSObject

+ (id)alloc {
    let host_object = Box::new(CALayerHostObject {
        delegate: nil,
        opaque: false,
        drawable_properties: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)layer {
    let new_layer: id = msg![env; this alloc];
    msg![env; new_layer init]
}

- (())dealloc {
    let &CALayerHostObject { drawable_properties, .. } = env.objc.borrow(this);
    if drawable_properties != nil {
        release(env, drawable_properties);
    }
}

- (id)delegate {
    env.objc.borrow::<CALayerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<CALayerHostObject>(this).delegate = delegate;
}

- (bool)isOpaque {
    env.objc.borrow::<CALayerHostObject>(this).opaque
}
- (())setOpaque:(bool)opaque {
    env.objc.borrow_mut::<CALayerHostObject>(this).opaque = opaque;
}

// TODO

@end

};
