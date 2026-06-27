/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSHashTable` — Foundation's unordered, pointer-personality–aware
//! companion to `NSSet`. Apple ships it in `<Foundation/NSHashTable.h>`.
//!
//! In Apple's reference implementation `NSHashTable` lets callers store
//! arbitrary pointers (or objects) under a configurable set of
//! `NSPointerFunctions` options that govern equality, hashing, retain /
//! release, copy and zeroing-weak semantics. The most common usage by
//! far is `+hashTableWithOptions:` with one of the documented option
//! masks, plus the convenience constructor
//! `+weakObjectsHashTable`, which is an Objective-C `NSSet`-like
//! collection that holds zeroing-weak references to its members.
//!
//! touchHLE does not ship a zeroing-weak runtime, so weak references
//! degrade to strong ones — but `NSHashTable`'s public surface
//! (`-count`, `-anyObject`, `-addObject:`, `-removeObject:`,
//! `-containsObject:`, `-allObjects`, `-objectEnumerator`,
//! `NSFastEnumeration`) is exactly the strong-reference subset that
//! `NSMutableSet` implements. We therefore back `NSHashTable` with the
//! same `DictionaryHostObject` machinery used by `NSSet`, ignore the
//! options mask, and let the rest of the cluster fall through to
//! `NSMutableSet`.
//!
//! References:
//! - <https://developer.apple.com/documentation/foundation/nshashtable>
//! - <https://developer.apple.com/documentation/foundation/nspointerfunctions>

use super::ns_dictionary::DictionaryHostObject;
use super::ns_enumerator::{fast_enumeration_helper, NSFastEnumerationState};
use super::NSUInteger;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

// `NSPointerFunctionsOptions` (subset). Apple `<Foundation/NSPointerFunctions.h>`.
// touchHLE does not switch behaviour on these — `NSHashTable` is always
// strong-personality-by-object — so we expose them for the public type
// signatures only.
pub type NSHashTableOptions = NSUInteger;

#[derive(Debug, Default)]
struct HashTableHostObject {
    dict: DictionaryHostObject,
    options: NSHashTableOptions,
}
impl HostObject for HashTableHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSHashTable: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    if this != env.objc.get_known_class("NSHashTable", &mut env.mem) {
        log!(
            "Warning: +[{:?} allocWithZone:{:?}] called on NSHashTable subclass; \
             falling back to NSHashTable.",
            this, zone
        );
    }
    let host_object = Box::<HashTableHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)hashTableWithOptions:(NSHashTableOptions)options {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithOptions:options capacity:0u32];
    autorelease(env, new)
}

+ (id)weakObjectsHashTable {
    // `NSPointerFunctionsWeakMemory == 5`. Strong-equality / object
    // personality is implicit. See Apple's NSPointerFunctions.h.
    msg![env; this hashTableWithOptions:5u32]
}

- (id)init {
    msg![env; this initWithOptions:0u32 capacity:0u32]
}

- (id)initWithOptions:(NSHashTableOptions)options
             capacity:(NSUInteger)_capacity {
    env.objc.borrow_mut::<HashTableHostObject>(this).options = options;
    this
}

- (())dealloc {
    let mut host_obj: HashTableHostObject =
        std::mem::take(env.objc.borrow_mut(this));
    host_obj.dict.release(env);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Querying

- (NSUInteger)count {
    env.objc.borrow::<HashTableHostObject>(this).dict.count
}

- (bool)containsObject:(id)object {
    if object == nil {
        return false;
    }
    let host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    let dict = host.dict;
    let res = dict.lookup(env, object) != nil;
    let options = host.options;
    *env.objc.borrow_mut(this) = HashTableHostObject { dict, options };
    res
}

- (id)member:(id)object {
    if object == nil {
        return nil;
    }
    let host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    let dict = host.dict;
    let found = dict.lookup(env, object);
    let res = if found != nil { object } else { nil };
    let options = host.options;
    *env.objc.borrow_mut(this) = HashTableHostObject { dict, options };
    res
}

- (id)anyObject {
    let host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    let res = host.dict.map.values()
        .flatten()
        .next()
        .map(|&(k, _)| k)
        .unwrap_or(nil);
    *env.objc.borrow_mut(this) = host;
    res
}

- (id)allObjects {
    let host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    let keys: Vec<id> = host.dict.map.values()
        .flatten()
        .map(|&(k, _)| k)
        .collect();
    *env.objc.borrow_mut(this) = host;
    for &k in &keys {
        retain(env, k);
    }
    let arr = crate::frameworks::foundation::ns_array::from_vec(env, keys);
    autorelease(env, arr)
}

- (id)objectEnumerator {
    let arr: id = msg![env; this allObjects];
    msg![env; arr objectEnumerator]
}

- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    let objects: id = msg![env; this allObjects];
    let count: NSUInteger = msg![env; objects count];
    fast_enumeration_helper(env, this, |env, idx| {
        if idx < count {
            msg![env; objects objectAtIndex:idx]
        } else {
            nil
        }
    }, state, stackbuf, len)
}

// MARK: - Mutation

- (())addObject:(id)object {
    if object == nil {
        return;
    }
    let null: id = msg_class![env; NSNull null];
    let mut host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    host.dict.insert(env, object, null, /* copy_key: */ false);
    *env.objc.borrow_mut(this) = host;
}

- (())removeObject:(id)object {
    if object == nil {
        return;
    }
    let mut host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    host.dict.remove(env, object);
    *env.objc.borrow_mut(this) = host;
}

- (())removeAllObjects {
    let mut host: HashTableHostObject = std::mem::take(env.objc.borrow_mut(this));
    host.dict.release(env);
    *env.objc.borrow_mut(this) = HashTableHostObject {
        dict: <DictionaryHostObject as Default>::default(),
        options: host.options,
    };
}

@end

};

// Suppress dead-code warnings for the strong-/weak-options helpers we
// re-export through the Objective-C bridge.
#[allow(dead_code)]
const _: fn() = || {
    let _ = release as fn(&mut crate::Environment, id);
};
