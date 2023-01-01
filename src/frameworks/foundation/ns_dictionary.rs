//! The `NSDictionary` class cluster, including `NSMutableDictionary`.

use super::NSUInteger;
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
};
use crate::Environment;
use std::collections::HashMap;

/// Alias for the return type of the `hash` method of the `NSObject` protocol.
type Hash = NSUInteger;

/// Belongs to _touchHLE_NSDictionary
#[derive(Debug, Default)]
struct DictionaryHostObject {
    /// Since we need custom hashing and custom equality, and these both need a
    /// `&mut Environment`, we can't just use a `HashMap<id, id>`.
    /// So here we are using a `HashMap` as a primitive for implementing a
    /// hash-map, which is not ideally efficient. :)
    /// The keys are the hash values, the values are a list of key-value pairs
    /// where the keys have the same hash value.
    map: HashMap<Hash, Vec<(id, id)>>,
    count: NSUInteger,
}
impl HostObject for DictionaryHostObject {}
impl DictionaryHostObject {
    fn lookup(&self, env: &mut Environment, key: id) -> id {
        let hash: Hash = msg![env; key hash];
        let Some(collisions) = self.map.get(&hash) else {
            return nil;
        };
        for &(candidate_key, value) in collisions {
            if candidate_key == key || msg![env; candidate_key isEqualTo:key] {
                return value;
            }
        }
        nil
    }
    fn insert(&mut self, env: &mut Environment, key: id, value: id) {
        let key: id = msg![env; key copy];
        let hash: Hash = msg![env; key hash];

        let value = retain(env, value);

        let Some(collisions) = self.map.get_mut(&hash) else {
            self.map.insert(hash, vec![(key, value)]);
            self.count += 1;
            return;
        };
        for &mut (candidate_key, ref mut existing_value) in collisions.iter_mut() {
            if candidate_key == key || msg![env; candidate_key isEqualTo:key] {
                release(env, *existing_value);
                *existing_value = value;
                return;
            }
        }
        collisions.push((key, value));
        self.count += 1;
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSDictionary is an abstract class. A subclass must provide:
// - (id)initWithObjects:(id*)forKeys:(id*)count:(NSUInteger)
// - (NSUInteger)count
// - (id)objectForKey:(id)
// - (NSEnumerator*)keyEnumerator
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSDictionary.
@implementation NSDictionary: NSObject

+ (id)allocWithZone:(MutVoidPtr)zone {
    // NSDictionary might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSDictionary", &mut env.mem));
    msg_class![env; _touchHLE_NSDictionary allocWithZone:zone]
}

+ (id)dictionaryWithObjectsAndKeys:(id)first_object /*, ...*/ {
    // This passes on the va_args by creative abuse of untyped function calls.
    // I should be ashamed, and you should be careful.
    let new_dict: id = msg![env; this alloc];
    let new_dict: id = msg![env; new_dict initWithObjectsAndKeys:first_object];
    autorelease(env, new_dict)
}

// NSCopying implementation
- (id)copyWithZone:(MutVoidPtr)_zone {
    // TODO: override this once we have NSMutableString!
    retain(env, this)
}

// TODO

@end

// Our private subclass that is the single implementation of NSDictionary for
// the time being.
@implementation _touchHLE_NSDictionary: NSDictionary

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(<DictionaryHostObject as Default>::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let obj = std::mem::take(env.objc.borrow_mut::<DictionaryHostObject>(this));
    for collisions in obj.map.values() {
        for &(key, value) in collisions {
            release(env, key);
            release(env, value);
        }
    }

    // FIXME: this should do a super-call instead
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)initWithObjectsAndKeys:(id)first_object, ...va_args {
    let first_key: id = va_args.next(env);
    assert!(first_key != nil); // TODO: raise proper exception

    let mut host_object = <DictionaryHostObject as Default>::default();
    host_object.insert(env, first_key, first_object);

    loop {
        let object: id = va_args.next(env);
        if object == nil {
            break;
        }
        let key: id = va_args.next(env);
        assert!(key != nil); // TODO: raise proper exception
        host_object.insert(env, key, object);
    }

    *env.objc.borrow_mut(this) = host_object;

    this
}

// TODO: enumeration, more init methods, etc

- (NSUInteger)count {
    env.objc.borrow::<DictionaryHostObject>(this).count
}
- (id)objectForKey:(id)key {
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    let res = host_obj.lookup(env, key);
    *env.objc.borrow_mut(this) = host_obj;
    res
}

@end

};
