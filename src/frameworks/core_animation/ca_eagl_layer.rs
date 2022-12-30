//! `CAEAGLLayer`.

use crate::objc::{id, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CAEAGLLayer: CALayer

// EAGLDrawable implementation (the only one)
// TODO: drawableProperties (getter)
- (())setDrawableProperties:(id)_properties { // NSDictionary<NSString*, id>*
    // TODO: actually store the properties somewhere
}

@end

};
