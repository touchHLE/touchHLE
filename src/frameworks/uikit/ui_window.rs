//! `UIWindow`.

use crate::objc::{id, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("TODO: [{:?} initWithCoder:{:?}]", this, coder);
    this
}

// TODO

@end

};
