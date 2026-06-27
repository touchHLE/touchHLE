/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSOrderedSet` class cluster, including `NSMutableOrderedSet`.
//!
//! Apple's documentation:
//! - [NSOrderedSet](https://developer.apple.com/documentation/foundation/nsorderedset):
//!   "A static, ordered collection of unique objects."
//! - [NSMutableOrderedSet](https://developer.apple.com/documentation/foundation/nsmutableorderedset):
//!   "A dynamic, ordered collection of unique objects."
//!
//! Like NSArray, an ordered set keeps its elements in insertion order and
//! supports indexed access; like NSSet, each object can appear only once
//! (uniqueness is determined with `isEqual:`). Inserting an object that is
//! already a member has no effect.

use super::ns_enumerator::{fast_enumeration_helper, NSFastEnumerationState};
use super::{ns_array, ns_string, NSNotFound, NSUInteger};
use crate::mem::{ConstPtr, MutPtr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

/// Belongs to _touchHLE_NSOrderedSet and _touchHLE_NSMutableOrderedSet.
#[derive(Debug, Default)]
struct OrderedSetHostObject {
    /// Objects in insertion order. Each object is retained by this Vec and
    /// appears at most once (`isEqual:` uniqueness).
    array: Vec<id>,
}
impl HostObject for OrderedSetHostObject {}

/// Returns the index of `object` in `host`'s array using `isEqual:`
/// semantics, or `None` if it is not a member.
fn index_of(env: &mut Environment, this: id, object: id) -> Option<usize> {
    let array = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for (i, &candidate) in array.iter().enumerate() {
        if candidate == object {
            return Some(i);
        }
        let equal: bool = msg![env; candidate isEqual:object];
        if equal {
            return Some(i);
        }
    }
    None
}

/// Appends `object` to `this` if not already a member. Retains on insert.
fn append_unique(env: &mut Environment, this: id, object: id) {
    if object == nil {
        // Matches Apple: inserting nil raises NSInvalidArgumentException.
        // We log instead of unwinding, consistent with touchHLE's
        // NSException handling.
        log!("Warning: attempted to add nil to an NSOrderedSet; ignoring.");
        return;
    }
    if index_of(env, this, object).is_none() {
        retain(env, object);
        env.objc
            .borrow_mut::<OrderedSetHostObject>(this)
            .array
            .push(object);
    }
}

/// Copies the contents of an NSArray (or anything responding to `count` /
/// `objectAtIndex:`) into `this`, uniquing along the way.
fn init_from_array_like(env: &mut Environment, this: id, array: id) {
    if array == nil {
        return;
    }
    let count: NSUInteger = msg![env; array count];
    for i in 0..count {
        let object: id = msg![env; array objectAtIndex:i];
        append_unique(env, this, object);
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSOrderedSet is an abstract class (class cluster). Our single concrete
// implementation for both the immutable and mutable variants is
// _touchHLE_NSOrderedSet / _touchHLE_NSMutableOrderedSet.
@implementation NSOrderedSet: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    if this != env.objc.get_known_class("NSOrderedSet", &mut env.mem) {
        log!(
            "Warning: +[{:?} allocWithZone:] called on NSOrderedSet subclass; \
             falling back to _touchHLE_NSOrderedSet.",
            this
        );
    }
    msg_class![env; _touchHLE_NSOrderedSet allocWithZone:zone]
}

+ (id)orderedSet {
    let new: id = msg![env; this new];
    autorelease(env, new)
}

+ (id)orderedSetWithObject:(id)object {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithObject:object];
    autorelease(env, new)
}

+ (id)orderedSetWithObjects:(id)first_obj, ...args {
    let new: id = msg![env; this alloc];
    append_unique(env, new, first_obj);
    if first_obj != nil {
        let mut varargs = args.start();
        loop {
            let next_arg: id = varargs.next(env);
            if next_arg.is_null() {
                break;
            }
            append_unique(env, new, next_arg);
        }
    }
    autorelease(env, new)
}

+ (id)orderedSetWithObjects:(ConstPtr<id>)objects count:(NSUInteger)count {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithObjects:objects count:count];
    autorelease(env, new)
}

+ (id)orderedSetWithArray:(id)array { // NSArray*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithArray:array];
    autorelease(env, new)
}

+ (id)orderedSetWithSet:(id)set { // NSSet*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithSet:set];
    autorelease(env, new)
}

+ (id)orderedSetWithOrderedSet:(id)other { // NSOrderedSet*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithOrderedSet:other];
    autorelease(env, new)
}

// NSCopying: immutable ordered sets just retain themselves.
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

// NSMutableCopying
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    let new: id = msg_class![env; NSMutableOrderedSet alloc];
    let array = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for &object in &array {
        retain(env, object);
    }
    env.objc.borrow_mut::<OrderedSetHostObject>(new).array = array;
    new
}

@end

@implementation NSMutableOrderedSet: NSOrderedSet

+ (id)allocWithZone:(NSZonePtr)zone {
    if this != env.objc.get_known_class("NSMutableOrderedSet", &mut env.mem) {
        log!(
            "Warning: +[{:?} allocWithZone:] called on NSMutableOrderedSet \
             subclass; falling back to _touchHLE_NSMutableOrderedSet.",
            this
        );
    }
    msg_class![env; _touchHLE_NSMutableOrderedSet allocWithZone:zone]
}

+ (id)orderedSetWithCapacity:(NSUInteger)capacity {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCapacity:capacity];
    autorelease(env, new)
}

// NSCopying: a copy of a mutable ordered set is an immutable snapshot.
- (id)copyWithZone:(NSZonePtr)_zone {
    let new: id = msg_class![env; NSOrderedSet alloc];
    let array = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for &object in &array {
        retain(env, object);
    }
    env.objc.borrow_mut::<OrderedSetHostObject>(new).array = array;
    new
}

@end

// Our private concrete subclass implementing the immutable variant.
@implementation _touchHLE_NSOrderedSet: NSOrderedSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<OrderedSetHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (id)initWithObject:(id)object {
    append_unique(env, this, object);
    this
}

- (id)initWithObjects:(id)first_obj, ...args {
    append_unique(env, this, first_obj);
    if first_obj != nil {
        let mut varargs = args.start();
        loop {
            let next_arg: id = varargs.next(env);
            if next_arg.is_null() {
                break;
            }
            append_unique(env, this, next_arg);
        }
    }
    this
}

- (id)initWithObjects:(ConstPtr<id>)objects count:(NSUInteger)count {
    for i in 0..count {
        let object: id = env.mem.read(objects + i);
        append_unique(env, this, object);
    }
    this
}

- (id)initWithArray:(id)array { // NSArray*
    init_from_array_like(env, this, array);
    this
}

- (id)initWithSet:(id)set { // NSSet*
    if set != nil {
        let objects: id = msg![env; set allObjects];
        init_from_array_like(env, this, objects);
    }
    this
}

- (id)initWithOrderedSet:(id)other { // NSOrderedSet*
    init_from_array_like(env, this, other);
    this
}

- (())dealloc {
    let array = std::mem::take(&mut env.objc.borrow_mut::<OrderedSetHostObject>(this).array);
    for object in array {
        release(env, object);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Accessors (shared by mutable subclass via inheritance)

- (NSUInteger)count {
    env.objc.borrow::<OrderedSetHostObject>(this).array.len() as NSUInteger
}

- (id)objectAtIndex:(NSUInteger)index {
    let array = &env.objc.borrow::<OrderedSetHostObject>(this).array;
    match array.get(index as usize) {
        Some(&object) => object,
        None => {
            log!(
                "Warning: -[NSOrderedSet objectAtIndex:{}] out of bounds \
                 (count {}); returning nil.",
                index,
                array.len()
            );
            nil
        }
    }
}

- (id)firstObject {
    env.objc
        .borrow::<OrderedSetHostObject>(this)
        .array
        .first()
        .copied()
        .unwrap_or(nil)
}

- (id)lastObject {
    env.objc
        .borrow::<OrderedSetHostObject>(this)
        .array
        .last()
        .copied()
        .unwrap_or(nil)
}

- (NSUInteger)indexOfObject:(id)object {
    match index_of(env, this, object) {
        Some(i) => i as NSUInteger,
        None => NSNotFound as NSUInteger,
    }
}

- (bool)containsObject:(id)object {
    index_of(env, this, object).is_some()
}

// Apple: "An array containing the set's members, or an empty array if the
// set has no members." The array reflects the set's order.
- (id)array {
    let objects = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for &object in &objects {
        retain(env, object);
    }
    let array = ns_array::from_vec(env, objects);
    autorelease(env, array)
}

- (id)set {
    let array: id = msg![env; this array];
    msg_class![env; NSSet setWithArray:array]
}

- (id)objectEnumerator { // NSEnumerator*
    let array: id = msg![env; this array];
    msg![env; array objectEnumerator]
}

- (id)reverseObjectEnumerator { // NSEnumerator*
    let array: id = msg![env; this array];
    msg![env; array reverseObjectEnumerator]
}

- (bool)isEqualToOrderedSet:(id)other {
    if this == other {
        return true;
    }
    if other == nil {
        return false;
    }
    let this_count: NSUInteger = msg![env; this count];
    let other_count: NSUInteger = msg![env; other count];
    if this_count != other_count {
        return false;
    }
    for i in 0..this_count {
        let a: id = msg![env; this objectAtIndex:i];
        let b: id = msg![env; other objectAtIndex:i];
        let equal: bool = msg![env; a isEqual:b];
        if !equal {
            return false;
        }
    }
    true
}

- (id)description {
    let array: id = msg![env; this array];
    msg![env; array description]
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    let count: NSUInteger = msg![env; this count];
    fast_enumeration_helper(env, this, |env, idx| {
        if idx < count {
            msg![env; this objectAtIndex:idx]
        } else {
            nil
        }
    }, state, stackbuf, len)
}

@end

// Our private concrete subclass implementing the mutable variant. It
// inherits all reading methods from _touchHLE_NSOrderedSet via the same
// host object layout.
@implementation _touchHLE_NSMutableOrderedSet: _touchHLE_NSOrderedSet

- (id)initWithCapacity:(NSUInteger)capacity {
    env.objc
        .borrow_mut::<OrderedSetHostObject>(this)
        .array
        .reserve(capacity as usize);
    this
}

// NSCopying / NSMutableCopying overrides: snapshots, not self-retains.
- (id)copyWithZone:(NSZonePtr)_zone {
    let new: id = msg_class![env; NSOrderedSet alloc];
    let array = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for &object in &array {
        retain(env, object);
    }
    env.objc.borrow_mut::<OrderedSetHostObject>(new).array = array;
    new
}

// MARK: - Mutation
// Apple (`-addObject:`): "Appends a given object to the end of the mutable
// ordered set, if it is not already a member."

- (())addObject:(id)object {
    append_unique(env, this, object);
}

- (())addObjectsFromArray:(id)array { // NSArray*
    init_from_array_like(env, this, array);
}

- (())insertObject:(id)object atIndex:(NSUInteger)index {
    if object == nil {
        log!("Warning: -[NSMutableOrderedSet insertObject:atIndex:] with nil; ignoring.");
        return;
    }
    if index_of(env, this, object).is_some() {
        return;
    }
    let len = env.objc.borrow::<OrderedSetHostObject>(this).array.len();
    if (index as usize) > len {
        log!(
            "Warning: -[NSMutableOrderedSet insertObject:atIndex:{}] out of \
             bounds (count {}); ignoring.",
            index,
            len
        );
        return;
    }
    retain(env, object);
    env.objc
        .borrow_mut::<OrderedSetHostObject>(this)
        .array
        .insert(index as usize, object);
}

- (())removeObject:(id)object {
    if let Some(i) = index_of(env, this, object) {
        let removed = env.objc.borrow_mut::<OrderedSetHostObject>(this).array.remove(i);
        release(env, removed);
    }
}

- (())removeObjectAtIndex:(NSUInteger)index {
    let len = env.objc.borrow::<OrderedSetHostObject>(this).array.len();
    if (index as usize) >= len {
        log!(
            "Warning: -[NSMutableOrderedSet removeObjectAtIndex:{}] out of \
             bounds (count {}); ignoring.",
            index,
            len
        );
        return;
    }
    let removed = env.objc.borrow_mut::<OrderedSetHostObject>(this).array.remove(index as usize);
    release(env, removed);
}

- (())removeAllObjects {
    let array = std::mem::take(&mut env.objc.borrow_mut::<OrderedSetHostObject>(this).array);
    for object in array {
        release(env, object);
    }
}

- (())replaceObjectAtIndex:(NSUInteger)index withObject:(id)object {
    if object == nil {
        log!("Warning: -[NSMutableOrderedSet replaceObjectAtIndex:withObject:] with nil; ignoring.");
        return;
    }
    let len = env.objc.borrow::<OrderedSetHostObject>(this).array.len();
    if (index as usize) >= len {
        log!(
            "Warning: -[NSMutableOrderedSet replaceObjectAtIndex:{}] out of \
             bounds (count {}); ignoring.",
            index,
            len
        );
        return;
    }
    // Per Apple, the replacement object must not already be a member at a
    // different index (uniqueness invariant).
    if let Some(existing) = index_of(env, this, object) {
        if existing != index as usize {
            log!(
                "Warning: -[NSMutableOrderedSet replaceObjectAtIndex:withObject:] \
                 object already a member at index {}; ignoring.",
                existing
            );
            return;
        }
    }
    retain(env, object);
    let old = std::mem::replace(
        &mut env.objc.borrow_mut::<OrderedSetHostObject>(this).array[index as usize],
        object,
    );
    release(env, old);
}

- (())setObject:(id)object atIndex:(NSUInteger)index {
    // Apple: appends if index == count, otherwise replaces.
    let len = env.objc.borrow::<OrderedSetHostObject>(this).array.len();
    if (index as usize) == len {
        append_unique(env, this, object);
    } else {
        () = msg![env; this replaceObjectAtIndex:index withObject:object];
    }
}

- (())exchangeObjectAtIndex:(NSUInteger)idx1 withObjectAtIndex:(NSUInteger)idx2 {
    let len = env.objc.borrow::<OrderedSetHostObject>(this).array.len();
    if (idx1 as usize) >= len || (idx2 as usize) >= len {
        log!(
            "Warning: -[NSMutableOrderedSet exchangeObjectAtIndex:{}:withObjectAtIndex:{}] \
             out of bounds (count {}); ignoring.",
            idx1,
            idx2,
            len
        );
        return;
    }
    env.objc
        .borrow_mut::<OrderedSetHostObject>(this)
        .array
        .swap(idx1 as usize, idx2 as usize);
}

- (())unionOrderedSet:(id)other { // NSOrderedSet*
    init_from_array_like(env, this, other);
}

- (())unionSet:(id)other { // NSSet*
    if other != nil {
        let objects: id = msg![env; other allObjects];
        init_from_array_like(env, this, objects);
    }
}

- (())minusOrderedSet:(id)other { // NSOrderedSet*
    if other == nil {
        return;
    }
    let count: NSUInteger = msg![env; other count];
    for i in 0..count {
        let object: id = msg![env; other objectAtIndex:i];
        () = msg![env; this removeObject:object];
    }
}

- (())minusSet:(id)other { // NSSet*
    if other == nil {
        return;
    }
    let objects: id = msg![env; other allObjects];
    let count: NSUInteger = msg![env; objects count];
    for i in 0..count {
        let object: id = msg![env; objects objectAtIndex:i];
        () = msg![env; this removeObject:object];
    }
}

- (())intersectOrderedSet:(id)other { // NSOrderedSet*
    let array = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for object in array {
        let keep: bool = if other == nil {
            false
        } else {
            msg![env; other containsObject:object]
        };
        if !keep {
            () = msg![env; this removeObject:object];
        }
    }
}

- (())intersectSet:(id)other { // NSSet*
    let array = env.objc.borrow::<OrderedSetHostObject>(this).array.clone();
    for object in array {
        let keep: bool = if other == nil {
            false
        } else {
            msg![env; other containsObject:object]
        };
        if !keep {
            () = msg![env; this removeObject:object];
        }
    }
}

@end

};

/// Helper for tests / host code: builds an NSString-keyed description for
/// debugging without going through the ObjC dispatcher.
#[allow(dead_code)]
pub fn debug_description(env: &mut Environment, ordered_set: id) -> String {
    let array = env
        .objc
        .borrow::<OrderedSetHostObject>(ordered_set)
        .array
        .clone();
    let mut out = String::from("{(\n");
    for object in array {
        let desc: id = msg![env; object description];
        out.push_str(&format!(
            "    {},\n",
            ns_string::to_rust_string(env, desc)
        ));
    }
    out.push_str(")}");
    out
}
