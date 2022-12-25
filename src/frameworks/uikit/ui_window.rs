//! `UIWindow`.

use crate::objc::{id, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIWindow*){:?} initWithCoder:{:?}]", this, coder);
    this
}

// TODO

@end

};
