/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSSet` class cluster, including `NSMutableSet` and `NSCountedSet`.

use super::ns_array;
use super::ns_dictionary::DictionaryHostObject;
use super::ns_enumerator::NSFastEnumerationState;
use super::NSUInteger;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;

/// Belongs to _touchHLE_NSSet
#[derive(Debug, Default)]
struct SetHostObject {
    dict: DictionaryHostObject,
}
impl HostObject for SetHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSSet is an abstract class. A subclass must provide:
// - (NSUInteger)count;
// - (id)member:(id)object;
// - (NSEnumerator*)objectEnumerator;
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSSet.
@implementation NSSet: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSSet might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSSet", &mut env.mem));
    msg_class![env; _touchHLE_NSSet allocWithZone:zone]
}

+ (id)setWithObject:(id)object {
    assert!(object != nil);
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithObject:object];
    autorelease(env, new)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

@end

// NSMutableSet is an abstract class. A subclass must provide everything
// NSSet provides, plus:
// - (void)addObject:(id)object;
// - (void)removeObject:(id)object;
// Note that it inherits from NSSet, so we must ensure we override any default
// methods that would be inappropriate for mutability.
@implementation NSMutableSet: NSSet

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSSet might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSMutableSet", &mut env.mem));
    msg_class![env; _touchHLE_NSMutableSet allocWithZone:zone]
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    todo!(); // TODO: this should produce an immutable copy
}

@end

// Our private subclass that is the single implementation of NSSet for the
// time being.
@implementation _touchHLE_NSSet: NSSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SetHostObject {
        dict: Default::default(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithObject:(id)object {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    dict.insert(env, object, null, /* copy_key: */ false);

    env.objc.borrow_mut::<SetHostObject>(this).dict = dict;

    this
}

- (())dealloc {
    std::mem::take(&mut env.objc.borrow_mut::<SetHostObject>(this).dict).release(env);
    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: more init methods, etc

// TODO: accessors
- (NSUInteger)count {
    env.objc.borrow_mut::<SetHostObject>(this).dict.count
}

- (id)anyObject {
    let object_or_none = env.objc.borrow_mut::<SetHostObject>(this).dict.iter_keys().next();
    match object_or_none {
        Some(object) => object,
        None => nil
    }
}

- (id)allObjects {
    let objects = env.objc.borrow_mut::<SetHostObject>(this).dict.iter_keys().collect();
    ns_array::from_vec(env, objects)
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    fast_enumeration_helper(env, this, state, stackbuf, len)
}

@end

// Our private subclass that is the single implementation of NSMutableSet for
// the time being.
@implementation _touchHLE_NSMutableSet: NSMutableSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SetHostObject {
        dict: Default::default(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithObject:(id)object {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    dict.insert(env, object, null, /* copy_key: */ false);

    env.objc.borrow_mut::<SetHostObject>(this).dict = dict;

    this
}

- (())dealloc {
    std::mem::take(&mut env.objc.borrow_mut::<SetHostObject>(this).dict).release(env);
    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: init methods etc

- (NSUInteger)count {
    env.objc.borrow_mut::<SetHostObject>(this).dict.count
}

- (id)anyObject {
    let object_or_none = env.objc.borrow_mut::<SetHostObject>(this).dict.iter_keys().next();
    match object_or_none {
        Some(object) => object,
        None => nil
    }
}

- (id)allObjects {
    let objects = env.objc.borrow_mut::<SetHostObject>(this).dict.iter_keys().collect();
    ns_array::from_vec(env, objects)
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    fast_enumeration_helper(env, this, state, stackbuf, len)
}

// TODO: more mutation methods

- (())addObject:(id)object {
    let null: id = msg_class![env; NSNull null];
    let mut host_obj: SetHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.dict.insert(env, object, null, /* copy_key: */ false);
    *env.objc.borrow_mut(this) = host_obj;
}

@end

};

fn fast_enumeration_helper(
    env: &mut Environment,
    set: id,
    state: MutPtr<NSFastEnumerationState>,
    stackbuf: MutPtr<id>,
    len: NSUInteger,
) -> NSUInteger {
    let host_object = env.objc.borrow::<SetHostObject>(set);

    if host_object.dict.count == 0 {
        return 0;
    }

    let NSFastEnumerationState {
        state: start_index, ..
    } = env.mem.read(state);

    let mut set_iter = host_object.dict.iter_keys();
    if start_index >= 1 {
        // FIXME: linear time complexity
        _ = set_iter.nth((start_index - 1).try_into().unwrap());
    }

    let mut batch_count = 0;
    while batch_count < len {
        if let Some(object) = set_iter.next() {
            env.mem.write(stackbuf + batch_count, object);
            batch_count += 1;
        } else {
            break;
        }
    }
    env.mem.write(
        state,
        NSFastEnumerationState {
            state: start_index + batch_count,
            items_ptr: stackbuf,
            // can be anything as long as it's dereferenceable and the same
            // each iteration
            // Note: stackbuf can be different each time, it's better to return
            // self pointer
            mutations_ptr: set.cast(),
            extra: Default::default(),
        },
    );
    batch_count
}
