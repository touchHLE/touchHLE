/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSDictionary` class cluster, including `NSMutableDictionary`.

use super::ns_array::ArrayHostObject;
use super::ns_property_list_serialization::{
    deserialize_plist_from_file, NSPropertyListBinaryFormat_v1_0,
};
use super::ns_string::{from_rust_string, to_rust_string};
use super::{ns_string, ns_url, NSUInteger};
use crate::abi::VaList;
use crate::fs::GuestPath;
use crate::mem::{MutPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;
use std::collections::HashMap;

/// Alias for the return type of the `hash` method of the `NSObject` protocol.
type Hash = NSUInteger;

/// Belongs to _touchHLE_NSDictionary, also used by _touchHLE_NSSet
#[derive(Debug, Default)]
pub(super) struct DictionaryHostObject {
    /// Since we need custom hashing and custom equality, and these both need a
    /// `&mut Environment`, we can't just use a `HashMap<id, id>`.
    /// So here we are using a `HashMap` as a primitive for implementing a
    /// hash-map, which is not ideally efficient. :)
    /// The keys are the hash values, the values are a list of key-value pairs
    /// where the keys have the same hash value.
    pub(super) map: HashMap<Hash, Vec<(id, id)>>,
    pub(super) count: NSUInteger,
}
impl HostObject for DictionaryHostObject {}
impl DictionaryHostObject {
    pub(super) fn lookup(&self, env: &mut Environment, key: id) -> id {
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
    pub(super) fn insert(&mut self, env: &mut Environment, key: id, value: id, copy_key: bool) {
        let key: id = if copy_key {
            msg![env; key copy]
        } else {
            retain(env, key)
        };
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
    pub(super) fn release(&mut self, env: &mut Environment) {
        for collisions in self.map.values() {
            for &(key, value) in collisions {
                release(env, key);
                release(env, value);
            }
        }
    }
    pub(super) fn iter_keys(&self) -> impl Iterator<Item = id> + '_ {
        self.map.values().flatten().map(|&(key, _value)| key)
    }
}

/// Helper to enable sharing `dictionaryWithObjectsAndKeys:` and
/// `initWithObjectsAndKeys:`' implementations without vararg passthrough.
pub fn init_with_objects_and_keys(
    env: &mut Environment,
    this: id,
    first_object: id,
    mut va_args: VaList,
) -> id {
    let first_key: id = va_args.next(env);
    assert!(first_key != nil); // TODO: raise proper exception

    let mut host_object = <DictionaryHostObject as Default>::default();
    host_object.insert(env, first_key, first_object, /* copy_key: */ true);

    loop {
        let object: id = va_args.next(env);
        if object == nil {
            break;
        }
        let key: id = va_args.next(env);
        assert!(key != nil); // TODO: raise proper exception
        host_object.insert(env, key, object, /* copy_key: */ true);
    }

    *env.objc.borrow_mut(this) = host_object;

    this
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

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSDictionary might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSDictionary", &mut env.mem));
    msg_class![env; _touchHLE_NSDictionary allocWithZone:zone]
}

+ (id)dictionary {
    let new_dict: id = msg![env; this alloc];
    let new_dict: id = msg![env; new_dict init];
    autorelease(env, new_dict)
}

+ (id)dictionaryWithObject:(id)object forKey:(id)key {
    assert_ne!(key, nil); // TODO: raise proper exception

    let new_dict = dict_from_keys_and_objects(env, &[(key, object)]);
    autorelease(env, new_dict)
}

+ (id)dictionaryWithObjectsAndKeys:(id)first_object, ...dots {
    let new_dict: id = msg![env; this alloc];
    let new_dict = init_with_objects_and_keys(env, new_dict, first_object, dots.start());
    autorelease(env, new_dict)
}

// These probably comes from some category related to plists.
+ (id)dictionaryWithContentsOfFile:(id)path { // NSString*
    let path = ns_string::to_rust_string(env, path);
    let res = deserialize_plist_from_file(
        env,
        GuestPath::new(&path),
        /* array_expected: */ false,
    );
    autorelease(env, res)
}
+ (id)dictionaryWithContentsOfURL:(id)url { // NSURL*
    let path = ns_url::to_rust_path(env, url);
    let res = deserialize_plist_from_file(env, &path, /* array_expected: */ false);
    autorelease(env, res)
}

- (id)init {
    todo!("TODO: Implement [dictionary init] for custom subclasses")
}

// These probably comes from some category related to plists.
- (id)initWithContentsOfFile:(id)path { // NSString*
    release(env, this);
    let path = ns_string::to_rust_string(env, path);
    deserialize_plist_from_file(
        env,
        GuestPath::new(&path),
        /* array_expected: */ false,
    )
}
- (id)initWithContentsOfURL:(id)url { // NSURL*
    release(env, this);
    let path = ns_url::to_rust_path(env, url);
    deserialize_plist_from_file(env, &path, /* array_expected: */ false)
}

- (bool)writeToFile:(id)path // NSString*
         atomically:(bool)atomically {
    let error_desc: MutPtr<id> = Ptr::null();
    let data: id = msg_class![env; NSPropertyListSerialization
            dataFromPropertyList:this
                          format:NSPropertyListBinaryFormat_v1_0
                errorDescription:error_desc];
    let res = msg![env; data writeToFile:path atomically:atomically];
    log_dbg!(
        "[(NSDictionary *){:?} writeToFile:{:?} atomically:{}] -> {}",
        this,
        to_rust_string(env, path),
        atomically,
        res
    );
    res
}

// TODO

- (id)valueForKey:(id)key { // NSString*
    let key_str = to_rust_string(env, key);
    // TODO: strip '@' and call super
    assert!(!key_str.starts_with('@'));
    msg![env; this objectForKey:key]
}

@end

// NSMutableDictionary is an abstract class. A subclass must provide everything
// NSDictionary provides, plus:
// - (void)setObject:(id)object forKey:(id)key;
// - (void)removeObjectForKey:(id)key;
// Note that it inherits from NSDictionary, so we must ensure we override
// any default methods that would be inappropriate for mutability.
@implementation NSMutableDictionary: NSDictionary

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSDictionary might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSMutableDictionary", &mut env.mem));
    msg_class![env; _touchHLE_NSMutableDictionary allocWithZone:zone]
}

+ (id)dictionaryWithCapacity:(NSUInteger)capacity {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCapacity:capacity];
    autorelease(env, new)
}

@end

// Our private subclass that is the single implementation of NSDictionary for
// the time being.
@implementation _touchHLE_NSDictionary: NSDictionary

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<DictionaryHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    std::mem::take(env.objc.borrow_mut::<DictionaryHostObject>(this)).release(env);

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)initWithObjectsAndKeys:(id)first_object, ...dots {
    init_with_objects_and_keys(env, this, first_object, dots.start())
}

- (id)init {
    *env.objc.borrow_mut(this) = <DictionaryHostObject as Default>::default();
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

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

// NSMutableCopying implementation
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    let mut_dict: id = msg_class![env; NSMutableDictionary alloc];
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    for (k, v) in host_obj.map.values().flatten() {
        () = msg![env; mut_dict setObject:(*v) forKey:(*k)];
    }
    *env.objc.borrow_mut(this) = host_obj;
    mut_dict
}

- (id)description {
    build_description(env, this)
}

@end

// Our private subclass that is the single implementation of
// NSMutableDictionary for the time being.
@implementation _touchHLE_NSMutableDictionary: NSMutableDictionary

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<DictionaryHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    std::mem::take(env.objc.borrow_mut::<DictionaryHostObject>(this)).release(env);

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)initWithObjectsAndKeys:(id)first_object, ...dots {
    init_with_objects_and_keys(env, this, first_object, dots.start())
}

- (id)init {
    *env.objc.borrow_mut(this) = <DictionaryHostObject as Default>::default();
    this
}

- (id)initWithCapacity:(NSUInteger)_capacity {
    // TODO: capacity
    msg![env; this init]
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

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    let entries: Vec<_> =
        env.objc.borrow_mut::<DictionaryHostObject>(this).map.values().flatten().copied().collect();
    dict_from_keys_and_objects(env, &entries)
}

// NSMutableCopying implementation
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    let mut_dict: id = msg_class![env; NSMutableDictionary alloc];
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    for (k, v) in host_obj.map.values().flatten() {
        () = msg![env; mut_dict setObject:(*v) forKey:(*k)];
    }
    *env.objc.borrow_mut(this) = host_obj;
    mut_dict
}

- (())setObject:(id)object
         forKey:(id)key {
    // TODO: raise NSInvalidArgumentException
    assert_ne!(object, nil);
    // TODO: raise NSInvalidArgumentException
    assert_ne!(key, nil);
    let mut host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.insert(env, key, object, /* copy_key: */ true);
    *env.objc.borrow_mut(this) = host_obj;
}

- (())addEntriesFromDictionary:(id)other { // NSDictionary *
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(other));
    for (k, v) in host_obj.map.values().flatten() {
        () = msg![env; this setObject:(*v) forKey:(*k)];
    }
    *env.objc.borrow_mut(other) = host_obj;
}

- (id)description {
    build_description(env, this)
}

@end

// Special variant for use by CFDictionary with NULL callbacks: objects aren't
// necessarily Objective-C objects and won't be retained/released.
// TODO: refactor with lookup/insert methods to use callbacks
@implementation _touchHLE_NSMutableDictionary_non_retaining: _touchHLE_NSMutableDictionary

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)initWithObjectsAndKeys:(id)_first_object, ..._dots {
    todo!();
}
- (id)description {
    todo!();
}
- (id)copyWithZone:(NSZonePtr)_zone {
    todo!();
}
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    todo!();
}

- (id)objectForKey:(id)key {
    // TODO: use CFDictionaryHashCallBack
    let hash: Hash = key.to_bits();
    let host_obj: &mut DictionaryHostObject = env.objc.borrow_mut(this);
    let Some(collisions) = host_obj.map.get(&hash) else {
        return nil;
    };
    for &(candidate_key, value) in collisions {
        // TODO: use CFDictionaryEqualCallBack
        if candidate_key == key {
            return value;
        }
    }
    nil
}

- (id)valueForKey:(id)_key {
    panic!("Unexpected call to valueForKey: for _touchHLE_NSMutableDictionary_non_retaining object {:?}", this);
}

- (())setObject:(id)value
         forKey:(id)key {
    assert!(!key.is_null());
    // TODO: use CFDictionaryHashCallBack
    let hash: Hash = key.to_bits();
    let host_obj: &mut DictionaryHostObject = env.objc.borrow_mut(this);
    let Some(collisions) = host_obj.map.get_mut(&hash) else {
        host_obj.map.insert(hash, vec![(key, value)]);
        host_obj.count += 1;
        return;
    };
    for &mut (candidate_key, ref mut existing_value) in collisions.iter_mut() {
        // TODO: use CFDictionaryEqualCallBack
        if candidate_key == key {
            *existing_value = value;
            return;
        }
    }
    collisions.push((key, value));
    host_obj.count += 1;
}

- (())removeObjectForKey:(id)key {
    assert!(!key.is_null());
    // TODO: use CFDictionaryHashCallBack
    let hash: Hash = key.to_bits();
    let host_obj: &mut DictionaryHostObject = env.objc.borrow_mut(this);
    let Some(collisions) = host_obj.map.get_mut(&hash) else {
        return;
    };
    let idx = collisions.iter().position(|&(candidate_key, _)| {
        // TODO: use CFDictionaryEqualCallBack
        candidate_key == key
    }).unwrap();
    collisions.remove(idx);
    host_obj.count -= 1;
}

- (id)allKeys {
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    let keys: Vec<id> = host_obj.map.values().flatten().map(|&(key, _value)| key).collect();
    *env.objc.borrow_mut(this) = host_obj;

    let array: id = msg_class![env; _touchHLE_NSArray_non_retaining alloc];
    env.objc.borrow_mut::<ArrayHostObject>(array).array = keys;
    array
}

@end

};

/// Direct constructor for use by host code, similar to
/// `[[NSDictionary alloc] initWithObjectsAndKeys:]` but without variadics and
/// with a more intuitive argument order. Unlike [super::ns_array::from_vec],
/// this **does** copy and retain!
pub fn dict_from_keys_and_objects(env: &mut Environment, keys_and_objects: &[(id, id)]) -> id {
    let dict: id = msg_class![env; NSDictionary alloc];

    let mut host_object = <DictionaryHostObject as Default>::default();
    for &(key, object) in keys_and_objects {
        host_object.insert(env, key, object, /* copy_key: */ true);
    }
    *env.objc.borrow_mut(dict) = host_object;

    dict
}

/// A helper to build a description NSString
/// for a NSDictionary or a NSMutableDictionary.
fn build_description(env: &mut Environment, dict: id) -> id {
    // According to docs, this description should be formatted as property list.
    // But by the same docs, it's meant to be used for debugging purposes only.
    let desc: id = msg_class![env; NSMutableString new];
    let prefix: id = from_rust_string(env, "{\n".to_string());
    () = msg![env; desc appendString:prefix];
    release(env, prefix);
    let keys: Vec<id> = env
        .objc
        .borrow_mut::<DictionaryHostObject>(dict)
        .iter_keys()
        .collect();
    for key in keys {
        let key_desc: id = msg![env; key description];
        let value: id = msg![env; dict objectForKey:key];
        let val_desc: id = msg![env; value description];
        // TODO: respect nesting and padding
        let format = format!(
            "\t{} = {};\n",
            to_rust_string(env, key_desc),
            to_rust_string(env, val_desc)
        );
        let format = from_rust_string(env, format);
        () = msg![env; desc appendString:format];
        release(env, format);
    }
    let suffix: id = from_rust_string(env, "}".to_string());
    () = msg![env; desc appendString:suffix];
    release(env, suffix);
    let desc_imm = msg![env; desc copy];
    release(env, desc);
    autorelease(env, desc_imm)
}
