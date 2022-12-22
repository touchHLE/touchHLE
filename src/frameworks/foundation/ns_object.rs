//! `NSObject`, the root of most class hierarchies in Objective-C.

use crate::objc::ClassExports;

pub const CLASSES: ClassExports = crate::objc_classes! {
    @implementation NSObject
    // TODO
    @end
};
