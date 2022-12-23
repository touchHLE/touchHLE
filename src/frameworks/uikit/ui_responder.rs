//! `UIResponder`.

use crate::objc::{objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIResponder: NSObject
// TODO
@end

};
