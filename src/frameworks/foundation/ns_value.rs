//! The `NSValue` class cluster, including `NSNumber`.

use super::NSUInteger;
use crate::mem::MutVoidPtr;
use crate::objc::{autorelease, id, msg, objc_classes, retain, ClassExports, HostObject};

enum NSNumberHostObject {
    Bool(bool),
}
impl HostObject for NSNumberHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSValue is an abstract class. None of the things it should provide are
// implemented here yet (TODO).
@implementation NSValue: NSObject

// NSCopying implementation
- (id)copyWithZone:(MutVoidPtr)_zone {
    retain(env, this)
}

@end

// NSNumber is not an abstract class.
@implementation NSNumber: NSValue

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(NSNumberHostObject::Bool(false));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)numberWithBool:(bool)value {
    // TODO: for greater efficiency we could return a static-lifetime value

    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBool:value];
    autorelease(env, new)
}

// TODO: types other than booleans

- (id)initWithBool:(bool)value {
    *env.objc.borrow_mut::<NSNumberHostObject>(this) = NSNumberHostObject::Bool(
        value,
    );
    this
}

- (NSUInteger)hash {
    let &NSNumberHostObject::Bool(value) = env.objc.borrow(this);
    super::hash_helper(&value)
}

// TODO: accessors etc

@end

};
