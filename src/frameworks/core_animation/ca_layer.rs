//! `CALayer`.

use crate::objc::{id, msg, nil, objc_classes, ClassExports, HostObject};

struct CALayerHostObject {
    /// Possibly nil, usually a UIView. This is a weak reference.
    delegate: id,
    opaque: bool,
}
impl HostObject for CALayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CALayer: NSObject

+ (id)alloc {
    let host_object = Box::new(CALayerHostObject {
        delegate: nil,
        opaque: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)layer {
    let new_layer: id = msg![env; this alloc];
    msg![env; new_layer init]
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
