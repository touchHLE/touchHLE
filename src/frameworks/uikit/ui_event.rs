//! `UIEvent`.

use crate::objc::{objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIEvent: NSObject
// TODO
@end

};
