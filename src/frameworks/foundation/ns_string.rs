//! The `NSString` class cluster, including `NSMutableString`.

use crate::mem::MutVoidPtr;
use crate::objc::{id, msg_class, objc_classes, ClassExports, HostObject};
use crate::Environment;

/// Belongs to _touchHLE_NSString
/// This is an enum because we will have to support UTF-16 eventually
enum StringHostObject {
    UTF8(String),
}
impl HostObject for StringHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSString is an abstract class. A subclass must provide:
// - (NSUInteger)length;
// - (unichar)characterAtIndex:(NSUInteger)index;
// We can pick whichever subclass we want for the various init methods.
// For the time being, that will always be _touchHLE_NSString.
@implementation NSString: NSObject

+ (id)allocWithZone:(MutVoidPtr)zone {
    // NSString might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSString", &mut env.mem));
    msg_class![env; _touchHLE_NSString allocWithZone:zone]
}

@end

// Our private subclass that is the single implementation of NSString for the
// time being.
@implementation _touchHLE_NSString: NSString

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(StringHostObject::UTF8(String::new()));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: accessors, init methods, etc

@end

};

/// Shortcut for host code, roughly equivalent to `stringWithUTF8String:` in the
/// proper API.
pub fn string_with_rust_string(env: &mut Environment, from: String) -> id {
    let string: id = msg_class![env; _touchHLE_NSString alloc];
    let host_object: &mut StringHostObject = env.objc.borrow_mut(string);
    *host_object = StringHostObject::UTF8(from);
    string
}

/// Shortcut for host code, retrieves a string in UTF-8.
/// TODO: Try to avoid allocating a new String where possible.
pub fn copy_string(env: &mut Environment, string: id) -> String {
    // TODO: handle foreign subclasses of NSString
    let host_object: &mut StringHostObject = env.objc.borrow_mut(string);
    let StringHostObject::UTF8(utf8) = host_object;
    utf8.clone()
}
