//! `NSObject`, the root of most class hierarchies in Objective-C.

use crate::objc::ClassExports;

pub const CLASSES: ClassExports = crate::objc_classes! {

(env, this, _cmd);

@implementation NSObject

+ (()) alloc { // FIXME: return type should be id
    unimplemented!("[NSObject alloc]")
}

- (()) init { // FIXME: return type should be id
    unimplemented!("[[NSObject alloc] init]")
}

@end

};
