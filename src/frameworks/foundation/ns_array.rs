//! The `NSArray` class cluster, including `NSMutableArray`.

use super::ns_keyed_unarchiver;
use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject};

// Belongs to _touchHLE_NSArray
struct ArrayHostObject {
    array: Vec<id>,
}
impl HostObject for ArrayHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSArray is an abstract class. A subclass must provide:
// - (NSUInteger)count;
// - (id)objectAtIndex:(NSUInteger)index;
// We can pick whichever subclass we want for the various init methods.
// For the time being, that will always be _touchHLE_NSArray.
@implementation NSArray: NSObject

+ (id)allocWithZone:(MutVoidPtr)zone {
    // NSArray might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSArray", &mut env.mem));
    msg_class![env; _touchHLE_NSArray allocWithZone:zone]
}

@end

// Our private subclass that is the single implementation of NSArray for the
// time being.
@implementation _touchHLE_NSArray: NSArray

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(ArrayHostObject {
        array: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    // It seems that every NSArray item in an NSKeyedArchiver plist looks like:
    // {
    //   "$class" => (uid of NSArray class goes here),
    //   "NS.objects" => [
    //     // objects here
    //   ]
    // }
    // Presumably we need to call a `decodeFooBarForKey:` method on the NSCoder
    // here, passing in an NSString for "NS.objects". There is no method for
    // arrays though (maybe it's `decodeObjectForKey:`), and in any case
    // allocating an NSString here would be inconvenient, so let's just take a
    // shortcut.
    // FIXME: What if it's not an NSKeyedUnarchiver?
    let objects = ns_keyed_unarchiver::decode_current_array(env, coder);
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    assert!(host_object.array.is_empty());
    host_object.array = objects; // objects are already retained
    this
}

- (())dealloc {
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    let array = std::mem::take(&mut host_object.array);

    for object in array {
        let _: () = msg![env; object release];
    }

    // FIXME: this should do a super-call instead
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
