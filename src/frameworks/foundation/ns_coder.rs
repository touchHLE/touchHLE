//! `NSCoder`.

use crate::objc::{objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCoder: NSObject
// This is an abstract class
@end

};
