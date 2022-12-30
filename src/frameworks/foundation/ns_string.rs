//! The `NSString` class cluster, including `NSMutableString`.

use super::NSUInteger;
use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, msg_class, objc_classes, retain, Class, ClassExports, HostObject};
use crate::Environment;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    static_str_pool: HashMap<&'static str, id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_string
    }
}

/// Belongs to _touchHLE_NSString.
/// This is an enum because we will have to support UTF-16 eventually.
enum StringHostObject {
    UTF8(Cow<'static, str>),
}
impl HostObject for StringHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSString is an abstract class. A subclass must provide:
// - (NSUInteger)length;
// - (unichar)characterAtIndex:(NSUInteger)index;
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSString.
@implementation NSString: NSObject

+ (id)allocWithZone:(MutVoidPtr)zone {
    // NSString might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSString", &mut env.mem));
    msg_class![env; _touchHLE_NSString allocWithZone:zone]
}

- (NSUInteger)hash {
    // TODO: avoid copying
    super::hash_helper(&to_rust_string(env, this))
}
- (bool)isEqualTo:(id)other {
    if this == other {
        return true;
    }
    let class: Class = msg_class![env; NSString class];
    if !msg![env; other isKindOfClass:class] {
        return false;
    }
    // TODO: avoid copying
    to_rust_string(env, this) == to_rust_string(env, other)
}

// NSCopying implementation
- (id)copyWithZone:(MutVoidPtr)_zone {
    // TODO: override this once we have NSMutableString!
    retain(env, this)
}

@end

// Our private subclass that is the single implementation of NSString for the
// time being.
@implementation _touchHLE_NSString: NSString

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(StringHostObject::UTF8(Cow::Borrowed("")));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: accessors, init methods, etc

@end

// Specialised subclass for static-lifetime strings.
// See `get_static_str`.
@implementation _touchHLE_NSString_Static: _touchHLE_NSString

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(StringHostObject::UTF8(Cow::Borrowed("")));
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id) retain { this }
- (()) release {}
- (id) autorelease { this }

@end

};

/// Shortcut for host code: get an NSString corresponding to a `&'static str`,
/// which does not have to be released and is never deallocated.
pub fn get_static_str(env: &mut Environment, from: &'static str) -> id {
    if let Some(&existing) = State::get(env).static_str_pool.get(from) {
        existing
    } else {
        let new = msg_class![env; _touchHLE_NSString_Static alloc];
        *env.objc.borrow_mut(new) = StringHostObject::UTF8(Cow::Borrowed(from));
        State::get(env).static_str_pool.insert(from, new);
        new
    }
}

/// Shortcut for host code, roughly equivalent to
/// `[[NSString alloc] initWithUTF8String:]` in the proper API.
pub fn from_rust_string(env: &mut Environment, from: String) -> id {
    let string: id = msg_class![env; _touchHLE_NSString alloc];
    let host_object: &mut StringHostObject = env.objc.borrow_mut(string);
    *host_object = StringHostObject::UTF8(Cow::Owned(from));
    string
}

/// Shortcut for host code, provides a view of a string in UTF-8.
/// TODO: Try to avoid allocating a new String in more cases.
pub fn to_rust_string(env: &mut Environment, string: id) -> Cow<'static, str> {
    // TODO: handle foreign subclasses of NSString
    let host_object: &mut StringHostObject = env.objc.borrow_mut(string);
    let StringHostObject::UTF8(utf8) = host_object;
    utf8.clone()
}
