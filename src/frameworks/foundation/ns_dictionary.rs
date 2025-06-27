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
use super::ns_string::{from_rust_string, get_static_str, to_rust_string};
use super::{ns_array, ns_keyed_unarchiver, ns_string, ns_url, NSUInteger};
use crate::abi::{CallFromHost, GuestFunction, VaList};
use crate::frameworks::core_foundation::{CFHashCode, CFIndex};
use crate::frameworks::foundation::ns_file_manager::{NSFileModificationDate, NSFileSize};
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::{impl_HostObject_with_superclass, Environment};
use std::collections::hash_map::Entry;
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
            if candidate_key == key || msg![env; candidate_key isEqual:key] {
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
            if candidate_key == key || msg![env; candidate_key isEqual:key] {
                release(env, *existing_value);
                *existing_value = value;
                return;
            }
        }
        collisions.push((key, value));
        self.count += 1;
    }
    pub(super) fn remove(&mut self, env: &mut Environment, key: id) {
        let hash: Hash = msg![env; key hash];
        let Some(collisions) = self.map.get_mut(&hash) else {
            return;
        };
        let Some(idx) = collisions.iter().position(|&(candidate_key, _)| {
            candidate_key == key || msg![env; candidate_key isEqual:key]
        }) else {
            return;
        };
        let (existing_key, value) = collisions[idx];
        release(env, existing_key);
        release(env, value);
        collisions.remove(idx);
        self.count -= 1;
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

// TODO: move those definitions to cf_dictionary.rs
// Right now they are here because we're too tied to
// NSDictionary internals, but separation could be cleaner?
#[repr(C, packed)]
pub struct CFDictionaryKeyCallBacks {
    pub version: CFIndex,         // version
    pub retain: GuestFunction,    // const void *(*retain)(CFAllocatorRef, const void *value)
    pub release: GuestFunction,   // void (*release)(CFAllocatorRef alloc, const void *val)
    pub copy_desc: GuestFunction, // CFStringRef (*copy_desc)(const void *val)
    pub equal: GuestFunction,     // Boolean (*equal)(const void *val1, const void *val2)
    pub hash: GuestFunction,      // CFHashCode (*hash)(const void *val)
}
unsafe impl SafeRead for CFDictionaryKeyCallBacks {}

#[repr(C, packed)]
pub struct CFDictionaryValueCallBacks {
    pub version: CFIndex,         // version
    pub retain: GuestFunction,    // const void *(*retain)(CFAllocatorRef, const void *value)
    pub release: GuestFunction,   // void (*release)(CFAllocatorRef alloc, const void *val)
    pub copy_desc: GuestFunction, // CFStringRef (*copy_desc)(const void *val)
    pub equal: GuestFunction,     // Boolean (*equal)(const void *val1, const void *val2)
}
unsafe impl SafeRead for CFDictionaryValueCallBacks {}

/// The choice of implementing CFDictionary as subclass
/// of NSDictionary is not a hard truth but a reflection
/// on the omnipresence of current NSDictionary implementation
/// as base of NSSet or usage of internals for property lists.
/// It's probably desirable to implement NSDictionary _atop of_
/// CFDictionary instead, but this requires considerable
/// refactoring, which I'm not very comfortable to do on
/// partially tested codebase (we do not have ability right
/// now to test NS objects directly, only CF variants ;( )
/// See TODO comment on the impl too.
pub struct CFDictionaryHostObject {
    superclass: DictionaryHostObject,
    /// `CFDictionaryKeyCallBacks`
    key_callbacks: CFDictionaryKeyCallBacks,
    /// `CFDictionaryValueCallBacks`
    value_callbacks: CFDictionaryValueCallBacks,
}
impl_HostObject_with_superclass!(CFDictionaryHostObject);
impl Default for CFDictionaryHostObject {
    fn default() -> Self {
        CFDictionaryHostObject {
            superclass: Default::default(),
            key_callbacks: CFDictionaryKeyCallBacks {
                version: 0, // version is always 0
                retain: GuestFunction::null_ptr(),
                release: GuestFunction::null_ptr(),
                copy_desc: GuestFunction::null_ptr(),
                equal: GuestFunction::null_ptr(),
                hash: GuestFunction::null_ptr(),
            },
            value_callbacks: CFDictionaryValueCallBacks {
                version: 0, // version is always 0
                retain: GuestFunction::null_ptr(),
                release: GuestFunction::null_ptr(),
                copy_desc: GuestFunction::null_ptr(),
                equal: GuestFunction::null_ptr(),
            },
        }
    }
}
// TODO: Unify implementations of NSDictionary and CFDictionary
impl CFDictionaryHostObject {
    fn lookup(&self, env: &mut Environment, key: id) -> id {
        let hash = self.hash(env, key);
        let Some(collisions) = self.superclass.map.get(&hash) else {
            return nil;
        };
        for &(candidate_key, value) in collisions {
            if self.equal_keys(env, candidate_key, key) {
                return value;
            }
        }
        nil
    }
    fn insert(&mut self, env: &mut Environment, key: id, value: id) {
        let hash = self.hash(env, key);
        let key = self.retain_key(env, key);
        let value = self.retain_value(env, value);
        self.superclass.count += 1;
        if let Entry::Vacant(e) = self.superclass.map.entry(hash) {
            e.insert(vec![(key, value)]);
            return;
        };
        // remove if present (count will be decremented if necessary)
        self.remove(env, key);
        self.superclass
            .map
            .get_mut(&hash)
            .unwrap()
            .push((key, value));
    }
    fn remove(&mut self, env: &mut Environment, key: id) -> bool {
        let hash = self.hash(env, key);
        let Some(collisions) = self.superclass.map.get(&hash) else {
            return false;
        };
        let maybe_pos = collisions
            .iter()
            .position(|&(candidate_key, _)| self.equal_keys(env, candidate_key, key));
        if let Some(pos) = maybe_pos {
            let (existing_key, existing_value) =
                self.superclass.map.get_mut(&hash).unwrap().remove(pos);
            self.release_key(env, existing_key);
            self.release_value(env, existing_value);
            self.superclass.count -= 1;
            true
        } else {
            false
        }
    }
    // helpers
    fn hash(&self, env: &mut Environment, key: id) -> CFHashCode {
        let hash_func = self.key_callbacks.hash;
        if hash_func.to_ptr().is_null() {
            // use the pointer value as a hash code
            key.to_bits()
        } else {
            hash_func.call_from_host(env, (key,))
        }
    }
    fn equal_keys(&self, env: &mut Environment, key1: id, key2: id) -> bool {
        let equal_func = self.key_callbacks.equal;
        if equal_func.to_ptr().is_null() {
            // pointer equality
            key1 == key2
        } else {
            equal_func.call_from_host(env, (key1, key2))
        }
    }
    fn retain_key(&mut self, env: &mut Environment, key: id) -> id {
        let key_retain_func = self.key_callbacks.retain;
        if key_retain_func.to_ptr().is_null() {
            key
        } else {
            // TODO: custom dict allocator
            key_retain_func.call_from_host(env, (nil, key))
        }
    }
    fn release_key(&mut self, env: &mut Environment, key: id) {
        let key_release_func = self.key_callbacks.release;
        if !key_release_func.to_ptr().is_null() {
            // TODO: custom dict allocator
            key_release_func.call_from_host(env, (nil, key))
        }
    }
    fn retain_value(&mut self, env: &mut Environment, value: id) -> id {
        let value_retain_func = self.value_callbacks.retain;
        if value_retain_func.to_ptr().is_null() {
            value
        } else {
            // TODO: custom dict allocator
            value_retain_func.call_from_host(env, (nil, value))
        }
    }
    fn release_value(&mut self, env: &mut Environment, value: id) {
        let value_release_func = self.value_callbacks.release;
        if !value_release_func.to_ptr().is_null() {
            // TODO: custom dict allocator
            value_release_func.call_from_host(env, (nil, value))
        }
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

/// Helper function to share `initWithDictionary:` implementations
fn init_with_dictionary_common(env: &mut Environment, this: id, other_dict: id) -> id {
    let other_host_object: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(other_dict));

    let mut host_object = <DictionaryHostObject as Default>::default();

    for key in other_host_object.iter_keys() {
        let object = other_host_object.lookup(env, key);
        host_object.insert(env, key, object, /* copy_key: */ true);
    }

    *env.objc.borrow_mut(this) = host_object;
    *env.objc.borrow_mut(other_dict) = other_host_object;
    this
}

/// Helper function so share `initWithObjects:ForKeys:` implementations
fn init_with_objects_for_keys_common(env: &mut Environment, this: id, objects: id, keys: id) -> id {
    let keys_size: NSUInteger = msg![env; keys count];
    let objects_size: NSUInteger = msg![env; objects count];
    assert_eq!(keys_size, objects_size); // TODO: raise proper exception

    let mut host_object = <DictionaryHostObject as Default>::default();

    let objects_enumerator: id = msg![env; objects objectEnumerator];
    let keys_enumerator: id = msg![env; keys objectEnumerator];

    loop {
        let next_key: id = msg![env; keys_enumerator nextObject];
        let next_object: id = msg![env; objects_enumerator nextObject];
        if next_key == nil {
            assert_eq!(next_object, nil);
            break;
        }
        host_object.insert(env, next_key, next_object, /* copy_key: */ true);
    }
    *env.objc.borrow_mut(this) = host_object;
    this
}

/// Helper function to share `allKeys` implementations
fn all_keys_common(env: &mut Environment, this: id) -> id {
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    let keys: Vec<id> = host_obj
        .map
        .values()
        .flatten()
        .map(|&(key, _value)| key)
        .collect();
    *env.objc.borrow_mut(this) = host_obj;
    for &key in &keys {
        retain(env, key);
    }
    let res = ns_array::from_vec(env, keys);
    autorelease(env, res)
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

+ (id)dictionaryWithObjects:(id)objects //NSArray *
                    forKeys:(id)keys { //NSArray *
    let new_dict: id = msg![env; this alloc];
    let new_dict: id = msg![env; new_dict initWithObjects:objects forKeys:keys];

    autorelease(env, new_dict)
}

+ (id)dictionaryWithDictionary:(id)dict { // NSDictionary*
    let new_dict: id = msg![env; this alloc];
    let new_dict: id = msg![env; new_dict initWithDictionary:dict];

    autorelease(env, new_dict)
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

// NSDictionary(NSFileAttributes) category
// TODO: implement categories properly
- (id)fileModificationDate {
    let modif_date_key = get_static_str(env, NSFileModificationDate);
    msg![env; this objectForKey:modif_date_key]
}
- (u64)fileSize {
    let size_key = get_static_str(env, NSFileSize);
    let num = msg![env; this objectForKey:size_key];
    if num != nil {
        msg![env; num unsignedLongLongValue]
    } else {
        // GnuStep docs claiming to return NSNotFound here [ref](https://www.gnustep.org/resources/documentation/Developer/Base/Reference/NSFileManager.html#method$NSDictionary(NSFileAttributes)-fileSize)
        // But as seen on iPhone Simulator, it's returning 0 with an empty dict
        0
    }
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

- (id)initWithDictionary:(id)dictionary {
    init_with_dictionary_common(env, this, dictionary)
}

- (id)initWithObjects:(id)objects //NSArray *
              forKeys:(id)keys { //NSArray *
    init_with_objects_for_keys_common(env, this, objects, keys)
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

- (id)allKeys {
    all_keys_common(env, this)
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

- (id)initWithDictionary:(id)dictionary {
    init_with_dictionary_common(env, this, dictionary)
}

- (id)init {
    *env.objc.borrow_mut(this) = <DictionaryHostObject as Default>::default();
    this
}

- (id)initWithCapacity:(NSUInteger)_capacity {
    // TODO: capacity
    msg![env; this init]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    // It seems that every NSDictionary item in an NSKeyedArchiver plist
    // looks like:
    // {
    //   "$class" => (uid of NSArray class goes here),
    //   "NS.keys" => [
    //     // keys here
    //   ]
    //   "NS.objects" => [
    //     // objects here
    //   ]
    // }
    release(env, this);
    // FIXME: What if it's not an NSKeyedUnarchiver?
    let tuples = ns_keyed_unarchiver::decode_current_dict(env, coder);
    let dict = dict_from_keys_and_objects(env, &tuples);

    let mut_dict = msg![env; dict mutableCopy];
    release(env, dict);
    mut_dict
}

- (id)initWithObjects:(id)objects //NSArray *
              forKeys:(id)keys { //NSArray *
    init_with_objects_for_keys_common(env, this, objects, keys)
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

- (())setValue:(id)value
        forKey:(id)key { // NSString *
    // TODO: assert that key is a string when using key-value coding
    if value == nil {
        msg![env; this removeObjectForKey:key]
    } else {
        msg![env; this setObject:value forKey:key]
    }
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

- (())removeObjectForKey:(id)key {
    assert!(!key.is_null());
    let mut host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.remove(env, key);
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

- (id)allKeys {
    all_keys_common(env, this)
}

- (id)allValues {
    let host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    let values: Vec<id> = host_obj.map.values().flatten().map(|&(_key, value)| value).collect();
    *env.objc.borrow_mut(this) = host_obj;

    for &val in &values {
        retain(env, val);
    }
    let res = ns_array::from_vec(env, values);
    autorelease(env, res)
}

- (id)objectEnumerator { // NSEnumerator*
    let values: id = msg![env; this allValues];
    msg![env; values objectEnumerator]
}

@end

// Special variant for use by CFDictionary with NULL callbacks: objects aren't
// necessarily Objective-C objects and won't be retained/released.
// TODO: refactor with lookup/insert methods to use callbacks
@implementation _touchHLE_NSMutableDictionary_non_retaining: _touchHLE_NSMutableDictionary

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CFDictionaryHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// our custom init, not a part of API
- (id)initWithKeyCallbacks:(ConstPtr<CFDictionaryKeyCallBacks>)key_callbacks
         andValueCallbacks:(ConstPtr<CFDictionaryValueCallBacks>)value_callbacks {
    if !key_callbacks.is_null() {
        assert!(!value_callbacks.is_null());
        let host_object = env.objc.borrow_mut::<CFDictionaryHostObject>(this);
        host_object.key_callbacks = env.mem.read(key_callbacks);
        host_object.value_callbacks = env.mem.read(value_callbacks);
    };
    this
}

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
    let host_obj: CFDictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    let res = host_obj.lookup(env, key);
    *env.objc.borrow_mut(this) = host_obj;
    res
}

- (id)valueForKey:(id)_key {
    panic!("Unexpected call to valueForKey: for _touchHLE_NSMutableDictionary_non_retaining object {this:?}");
}

- (())setObject:(id)object
         forKey:(id)key {
    assert!(!key.is_null());
    let mut host_obj: CFDictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.insert(env, key, object);
    *env.objc.borrow_mut(this) = host_obj;
}

- (())removeObjectForKey:(id)key {
    assert!(!key.is_null());
    let mut host_obj: CFDictionaryHostObject = std::mem::take(env.objc.borrow_mut(this));
    host_obj.remove(env, key);
    *env.objc.borrow_mut(this) = host_obj;
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
