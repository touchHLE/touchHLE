//! `NSAutoreleasePool`.

use crate::objc::ClassExports;

pub const CLASSES: ClassExports = crate::objc_classes! {

(env, this, _cmd);

@implementation NSAutoreleasePool: NSObject
// TODO
@end

};
