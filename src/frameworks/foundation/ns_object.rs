//! `NSObject`, the root of most class hierarchies in Objective-C.

use crate::objc::{id, nil, ClassExports};

pub const CLASSES: ClassExports = crate::objc_classes! {

(env, this, _cmd);

@implementation NSObject

+ (id) alloc {
    nil // FIXME: return real object
}

- (id) init {
    unimplemented!("[[NSObject alloc] init]")
}

@end

};
