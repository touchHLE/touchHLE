/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSSet` class cluster, including `NSMutableSet` and `NSCountedSet`.

use crate::frameworks::foundation::ns_array::to_vec;
use super::ns_array;
use super::ns_dictionary::DictionaryHostObject;
use super::ns_enumerator::NSFastEnumerationState;
use super::NSUInteger;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, retain, ClassExports, HostObject, NSZonePtr,
};

/// Belongs to _touchHLE_NSSet
#[derive(Default, Debug)]
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

@implementation NSMutableSet: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSSet might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSMutableSet", &mut env.mem));
    msg_class![env; _touchHLE_NSSet allocWithZone:zone]
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    let objs: id = msg![env; this allObjects];
    let class = msg![env; this class];
    let new = msg![env; class alloc];
    msg![env; new initWithArray: objs]
}

@end

// Our private subclass that is the single implementation of NSSet for the
// time being.
@implementation _touchHLE_NSSet: NSMutableSet

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

- (id)initWithArray:(id)array {
    let null: id = msg_class![env; NSNull null];

    let mut dict = <DictionaryHostObject as Default>::default();
    let objects = to_vec(env, array);
    for object in objects {
        dict.insert(env, object, null, /* copy_key: */ false);
    }

    env.objc.borrow_mut::<SetHostObject>(this).dict = dict;

    this
}

- (())dealloc {
    std::mem::take(&mut env.objc.borrow_mut::<SetHostObject>(this).dict).release(env);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)initWithCapacity:(NSUInteger)cap {
    env.objc.borrow_mut::<SetHostObject>(this).dict.map.reserve(cap as usize);
    this
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
    let array = ns_array::from_vec(env, objects);
    autorelease(env, array)
}

- (bool)containsObject:(id)object {
    let host_obj = std::mem::take(env.objc.borrow_mut::<SetHostObject>(this));
    let res = host_obj.dict.lookup(env, object);
    *env.objc.borrow_mut(this) = host_obj;
    res != nil
}

- (())addObject:(id)object {
    let null: id = msg_class![env; NSNull null];
    let mut host_obj = std::mem::take(env.objc.borrow_mut::<SetHostObject>(this));
    host_obj.dict.insert(env, object, null, false);
    *env.objc.borrow_mut(this) = host_obj;
}

- (())removeObject:(id)object {
    let mut host_obj = std::mem::take(env.objc.borrow_mut::<SetHostObject>(this));
    host_obj.dict.remove(env, object);
    *env.objc.borrow_mut(this) = host_obj;
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    let host_object = env.objc.borrow::<SetHostObject>(this);

    if host_object.dict.count == 0 {
        return 0;
    }

    // TODO: handle size > 1
    assert!(host_object.dict.count == 1);
    assert!(len >= host_object.dict.count);

    let NSFastEnumerationState {
        state: is_first_round,
        ..
    } = env.mem.read(state);

    match is_first_round {
        0 => {
            let object = host_object.dict.iter_keys().next().unwrap();
            env.mem.write(stackbuf, object);
            env.mem.write(state, NSFastEnumerationState {
                state: 1,
                items_ptr: stackbuf,
                // can be anything as long as it's dereferenceable and the same
                // each iteration
                mutations_ptr: stackbuf.cast(),
                extra: Default::default(),
            });
            1 // returned object count
        },
        1 => {
            0 // end of iteration
        },
        _ => panic!(), // app failed to initialize the buffer?
    }
}

@end

};
