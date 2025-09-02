/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSArray` class cluster, including `NSMutableArray`.

use super::ns_enumerator::{fast_enumeration_helper, NSFastEnumerationState};
use super::ns_property_list_serialization::deserialize_plist_from_file;
use super::{ns_keyed_unarchiver, ns_string, ns_url, NSInteger, NSNotFound, NSRange, NSUInteger};
use crate::abi::{CallFromHost, GuestFunction};
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};
use crate::Environment;

struct ObjectEnumeratorHostObject {
    /// the enumerated collection, NSArray *
    array: id,
    /// an iterator
    iterator: std::vec::IntoIter<id>,
}
impl HostObject for ObjectEnumeratorHostObject {}

/// Belongs to _touchHLE_NSArray
#[derive(Debug, Default)]
pub(super) struct ArrayHostObject {
    pub(super) array: Vec<id>,
}
impl HostObject for ArrayHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSArray is an abstract class. A subclass must provide:
// - (NSUInteger)count;
// - (id)objectAtIndex:(NSUInteger)index;
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSArray.
@implementation NSArray: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSArray might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSArray", &mut env.mem));
    msg_class![env; _touchHLE_NSArray allocWithZone:zone]
}

+ (id)array {
    let array: id = msg![env; this new];
    autorelease(env, array)
}

+ (id)arrayWithArray:(id)other { // NSArray*
    let array: id = msg![env; this alloc];
    let array: id = msg![env; array initWithArray:other];
    autorelease(env, array)
}

// These probably comes from some category related to plists.
+ (id)arrayWithContentsOfFile:(id)path { // NSString*
    let path = ns_string::to_rust_string(env, path);
    let res = deserialize_plist_from_file(
        env,
        GuestPath::new(&path),
        /* array_expected: */ true,
    );
    autorelease(env, res)
}
+ (id)arrayWithContentsOfURL:(id)url { // NSURL*
    let path = ns_url::to_rust_path(env, url);
    let res = deserialize_plist_from_file(env, &path, /* array_expected: */ true);
    autorelease(env, res)
}
+ (id)arrayWithObject:(id)object {
    retain(env, object);
    let objects = vec![object];
    let array = from_vec(env, objects);
    autorelease(env, array)
}
+ (id)arrayWithObjects:(id)firstObj, ...args {
    retain(env, firstObj);
    let mut objects = vec![firstObj];
    let mut varargs = args.start();
    loop {
        let next_arg: id = varargs.next(env);
        if next_arg.is_null() {
            break;
        }
        retain(env, next_arg);
        objects.push(next_arg);
    }
    let array = from_vec(env, objects);
    autorelease(env, array)
}
+ (id)arrayWithObjects:(ConstPtr<id>)objects_ptr count:(NSUInteger)count {
    let array: id = msg![env; this alloc];
    let array: id = msg![env; array initWithObjects:objects_ptr count:count];
    autorelease(env, array)
}

// These probably comes from some category related to plists.
- (id)initWithContentsOfFile:(id)path { // NSString*
    release(env, this);
    let path = ns_string::to_rust_string(env, path);
    deserialize_plist_from_file(
        env,
        GuestPath::new(&path),
        /* array_expected: */ true,
    )
}
- (id)initWithContentsOfURL:(id)url { // NSURL*
    release(env, this);
    let path = ns_url::to_rust_path(env, url);
    deserialize_plist_from_file(env, &path, /* array_expected: */ true)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

- (NSUInteger)indexOfObject:(id)object {
    let count: NSUInteger = msg![env; this count];
    for i in 0..count {
        let curr_object: id = msg![env; this objectAtIndex:i];
        let equal: bool = msg![env; object isEqual:curr_object];
        if equal {
            return i;
        }
    }
    NSNotFound as NSUInteger
}
- (bool)containsObject:(id)object {
    let idx: NSUInteger = msg![env; this indexOfObject:object];
    idx != NSNotFound as NSUInteger
}

- (id)firstObject {
    let size: NSUInteger = msg![env; this count];
    if size == 0 {
        return nil;
    }
    msg![env; this objectAtIndex:0u32]
}

- (id)lastObject {
    let size: NSUInteger = msg![env; this count];
    if size == 0 {
        return nil;
    }
    msg![env; this objectAtIndex:(size - 1)]
}

- (id)componentsJoinedByString:(id)str { // NSString *
    let res: id = msg_class![env; NSMutableString new];
    let count: NSUInteger = msg![env; this count];
    if count == 0 {
        autorelease(env, res);
        return res;
    }
    for i in 0..count {
        let curr_object: id = msg![env; this objectAtIndex:i];
        let curr_desc: id = msg![env; curr_object description];
        () = msg![env; res appendString:curr_desc];
        if i != count-1 {
            () = msg![env; res appendString:str];
        }
    }
    let res_imm = msg![env; res copy];
    release(env, res);
    autorelease(env, res_imm)
}

@end

// NSMutableArray is an abstract class. A subclass must provide everything
// NSArray provides, plus:
// - (void)insertObject:(id)object atIndex:(NSUInteger)index;
// - (void)removeObjectAtIndex:(NSUInteger)index;
// - (void)addObject:(id)object;
// - (void)removeLastObject
// - (void)replaceObjectAtIndex:(NSUInteger)index withObject:(id)object;
// Note that it inherits from NSArray, so we must ensure we override any default
// methods that would be inappropriate for mutability.
@implementation NSMutableArray: NSArray

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSArray might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSMutableArray", &mut env.mem));
    msg_class![env; _touchHLE_NSMutableArray allocWithZone:zone]
}

+ (id)arrayWithCapacity:(NSUInteger)capacity {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCapacity:capacity];
    autorelease(env, new)
}

+ (id)arrayWithArray:(id)array {
    let new: id = msg![env; this alloc];
    () = msg![env; new addObjectsFromArray:array];
    autorelease(env, new)
}

+ (id)arrayWithObjects:(id)firstObj, ...args {
    retain(env, firstObj);
    let mut objects = vec![firstObj];
    let mut varargs = args.start();
    loop {
        let next_arg: id = varargs.next(env);
        if next_arg.is_null() {
            break;
        }
        retain(env, next_arg);
        objects.push(next_arg);
    }
    let array = from_vec_mut(env, objects);
    autorelease(env, array)
}

- (())addObjectsFromArray:(id)other { // NSArray*
    let enumerator: id = msg![env; other objectEnumerator];
    loop {
        let next: id = msg![env; enumerator nextObject];
        if next == nil {
            break;
        }
        () = msg![env; this addObject:next];
    }
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    let other: id = msg_class![env; NSArray alloc];
    let other: id = msg![env; other initWithArray:this];
    other
}

@end

// Our private subclass that is the single implementation of NSArray for the
// time being.
@implementation _touchHLE_NSArray: NSArray

+ (id)allocWithZone:(NSZonePtr)_zone {
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

- (id)initWithArray:(id)array { // NSArray*
    let mut objects = Vec::new();
    let enumerator: id = msg![env; array objectEnumerator];
    loop {
        let next: id = msg![env; enumerator nextObject];
        if next == nil {
            break;
        }
        objects.push(next);
        retain(env, next);
    }
    env.objc.borrow_mut::<ArrayHostObject>(this).array = objects;
    this
}

- (id)initWithObjects:(id)firstObj, ...args {
    retain(env, firstObj);
    let mut objects = vec![firstObj];
    let mut varargs = args.start();
    loop {
        let next_arg: id = varargs.next(env);
        if next_arg.is_null() {
            break;
        }
        retain(env, next_arg);
        objects.push(next_arg);
    }
    env.objc.borrow_mut::<ArrayHostObject>(this).array = objects;
    this
}

- (id)initWithObjects:(ConstPtr<id>)objects_ptr count:(NSUInteger)count {
    let mut objects = Vec::new();
    for i in 0..count {
        let obj: id = env.mem.read(objects_ptr + i);
        retain(env, obj);
        objects.push(obj);
    }
    env.objc.borrow_mut::<ArrayHostObject>(this).array = objects;
    this
}

- (())dealloc {
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    let array = std::mem::take(&mut host_object.array);

    for object in array {
        release(env, object);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)objectEnumerator { // NSEnumerator*
    object_enumerator_inner(env, this)
}
- (id)reverseObjectEnumerator { // NSEnumerator*
    reverse_object_enumerator_inner(env, this)
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

// TODO: more init methods, etc

- (NSUInteger)count {
    env.objc.borrow::<ArrayHostObject>(this).array.len().try_into().unwrap()
}
- (id)objectAtIndex:(NSUInteger)index {
    // TODO: throw real exception rather than panic if out-of-bounds?
    env.objc.borrow::<ArrayHostObject>(this).array[index as usize]
}

- (id)description {
    build_description(env, this)
}

- (id)subarrayWithRange:(NSRange)range {
    let mut tmp = Vec::new();
    tmp.extend_from_slice(
        &env.objc.borrow::<ArrayHostObject>(this).array[range.location as usize..(range.location + range.length) as usize]
    );
    for &obj in &tmp {
        retain(env, obj);
    }
    let res = from_vec(env, tmp);
    autorelease(env, res)
}

@end

// Special variant for use by CFArray with NULL callbacks: objects aren't
// necessarily Objective-C objects and won't be retained/released.
@implementation _touchHLE_NSArray_non_retaining: _touchHLE_NSArray

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation _touchHLE_NSArray_ObjectEnumerator: NSEnumerator

- (id)nextObject {
    let host_obj = env.objc.borrow_mut::<ObjectEnumeratorHostObject>(this);
    host_obj.iterator.next().map_or(nil, |o| o)
}

- (())dealloc {
    let host_obj = env.objc.borrow::<ObjectEnumeratorHostObject>(this);
    release(env, host_obj.array);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// Our private subclass that is the single implementation of NSMutableArray for
// the time being.
@implementation _touchHLE_NSMutableArray: NSMutableArray

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ArrayHostObject {
        array: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCapacity:(NSUInteger)capacity {
    env.objc.borrow_mut::<ArrayHostObject>(this).array.reserve(capacity as usize);
    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let objects = ns_keyed_unarchiver::decode_current_array(env, coder);
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    assert!(host_object.array.is_empty());
    host_object.array = objects; // objects are already retained
    this
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    let arr: id = msg_class![env; NSArray alloc];
    let array = env.objc.borrow::<ArrayHostObject>(this).array.clone();
    for &object in &array {
        retain(env, object);
    }
    env.objc.borrow_mut::<ArrayHostObject>(arr).array = array;
    arr
}

// NSMutableCopying implementation
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    let mut_arr: id = msg_class![env; NSMutableArray alloc];
    let array = env.objc.borrow::<ArrayHostObject>(this).array.clone();
    for &object in &array {
        retain(env, object);
    }
    env.objc.borrow_mut::<ArrayHostObject>(mut_arr).array = array;
    mut_arr
}

- (())dealloc {
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    let array = std::mem::take(&mut host_object.array);

    for object in array {
        release(env, object);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

- (())makeObjectsPerformSelector:(SEL)sel {
    let count: NSUInteger = msg![env; this count];
    for idx in 0..count {
        let obj: id = msg![env; this objectAtIndex:idx];
        let _: id = msg![env; obj performSelector:sel];
    }
}

- (id)objectEnumerator { // NSEnumerator*
    object_enumerator_inner(env, this)
}
- (id)reverseObjectEnumerator { // NSEnumerator*
    reverse_object_enumerator_inner(env, this)
}

- (())sortUsingFunction:(GuestFunction)comparator
                context:(MutVoidPtr)context {
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    let mut array = std::mem::take(&mut host_object.array);
    array.sort_by(|&a, &b| {
        let res: NSInteger = comparator.call_from_host(env, (a, b, context));
        res.cmp(&0)
    });

    env.objc.borrow_mut::<ArrayHostObject>(this).array = array;
}

// NSFastEnumeration implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    // TODO: check that array wasn't mutated!
    let count: NSUInteger = msg![env; this count];
    fast_enumeration_helper(env, this, |env, idx| {
        if idx < count {
            msg![env; this objectAtIndex:idx]
        } else {
            nil
        }
    }, state, stackbuf, len)
}

- (NSUInteger)count {
    env.objc.borrow::<ArrayHostObject>(this).array.len().try_into().unwrap()
}
- (id)objectAtIndex:(NSUInteger)index {
    // TODO: throw real exception rather than panic if out-of-bounds?
    env.objc.borrow::<ArrayHostObject>(this).array[index as usize]
}

- (id)description {
    build_description(env, this)
}

// TODO: more mutation methods

- (())insertObject:(id)object
           atIndex:(NSUInteger)index {
    retain(env, object);
    env.objc.borrow_mut::<ArrayHostObject>(this).array.insert(index as usize, object);
}

- (())addObject:(id)object {
    retain(env, object);
    env.objc.borrow_mut::<ArrayHostObject>(this).array.push(object);
}

- (())removeObject:(id)object {
    let mut to_remove = Vec::new();
    let count: NSUInteger = msg![env; this count];
    for i in 0..count {
        let curr_object: id = msg![env; this objectAtIndex:i];
        let equal: bool = msg![env; object isEqual:curr_object];
        if equal {
            to_remove.push(i);
        }
    }
    // TODO: runtime here is O(n^2), it could be O(n) instead
    for i in to_remove {
        () = msg![env; this removeObjectAtIndex:i];
    }
}

- (())removeObjectAtIndex:(NSUInteger)index {
    let object = env.objc.borrow_mut::<ArrayHostObject>(this).array.remove(index as usize);
    release(env, object)
}

- (())replaceObjectAtIndex:(NSUInteger)index withObject:(id)obj {
    retain(env, obj);
    let object = std::mem::replace(&mut env.objc.borrow_mut::<ArrayHostObject>(this).array[index as usize], obj);
    release(env, object);
}

- (())removeLastObject {
    let object = env.objc.borrow_mut::<ArrayHostObject>(this).array.pop().unwrap();
    release(env, object)
}

- (())removeAllObjects {
    let host_object: &mut ArrayHostObject = env.objc.borrow_mut(this);
    let array = std::mem::take(&mut host_object.array);
    for object in array {
        release(env, object);
    }

    env.objc.borrow_mut::<ArrayHostObject>(this).array = Vec::new()
}

@end

// Special variant for use by CFArray with NULL callbacks: objects aren't
// necessarily Objective-C objects and won't be retained/released.
@implementation _touchHLE_NSMutableArray_non_retaining: _touchHLE_NSMutableArray

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

- (())addObject:(id)object {
    env.objc.borrow_mut::<ArrayHostObject>(this).array.push(object);
}

- (())removeObjectAtIndex:(NSUInteger)index {
    env.objc.borrow_mut::<ArrayHostObject>(this).array.remove(index as usize);
}

- (())removeLastObject {
    env.objc.borrow_mut::<ArrayHostObject>(this).array.pop().unwrap();
}

@end

};

/// Shortcut for host code, roughly equivalent to
/// `[[NSArray alloc] initWithObjects:count]` but without copying.
/// The elements should already be "retained by" the `Vec`.
pub fn from_vec(env: &mut Environment, objects: Vec<id>) -> id {
    let array: id = msg_class![env; NSArray alloc];
    env.objc.borrow_mut::<ArrayHostObject>(array).array = objects;
    array
}

/// Shortcut for host code, roughly equivalent to
/// `[[NSMutableArray alloc] initWithObjects:count]` but without copying.
/// The elements should already be "retained by" the `Vec`.
pub fn from_vec_mut(env: &mut Environment, objects: Vec<id>) -> id {
    let array: id = msg_class![env; NSMutableArray alloc];
    env.objc.borrow_mut::<ArrayHostObject>(array).array = objects;
    array
}

/// A helper to build a description NSString
/// for a NSArray or a NSMutableArray.
fn build_description(env: &mut Environment, arr: id) -> id {
    // According to docs, this description should be formatted as property list.
    // But by the same docs, it's meant to be used for debugging purposes only.
    let desc: id = msg_class![env; NSMutableString new];
    let prefix: id = ns_string::from_rust_string(env, "(\n".to_string());
    () = msg![env; desc appendString:prefix];
    release(env, prefix);
    let values: Vec<id> = env.objc.borrow_mut::<ArrayHostObject>(arr).array.clone();
    for value in values {
        let value_desc: id = msg![env; value description];
        // TODO: respect nesting and padding
        let format = format!("\t{},\n", ns_string::to_rust_string(env, value_desc));
        let format = ns_string::from_rust_string(env, format);
        () = msg![env; desc appendString:format];
        release(env, format);
    }
    let suffix: id = ns_string::from_rust_string(env, ")".to_string());
    () = msg![env; desc appendString:suffix];
    release(env, suffix);
    let desc_imm = msg![env; desc copy];
    release(env, desc);
    autorelease(env, desc_imm)
}

/// A shared objectEnumerator helper method.
fn object_enumerator_inner(env: &mut Environment, arr: id) -> id {
    let array_host_object: &mut ArrayHostObject = env.objc.borrow_mut(arr);
    let vec = array_host_object.array.to_vec();
    object_enumerator_inner_helper(env, arr, vec)
}

/// A shared reverseObjectEnumerator helper method.
fn reverse_object_enumerator_inner(env: &mut Environment, arr: id) -> id {
    let array_host_object: &mut ArrayHostObject = env.objc.borrow_mut(arr);
    // TODO: avoid copying?
    let vec = array_host_object
        .array
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>();
    object_enumerator_inner_helper(env, arr, vec)
}

fn object_enumerator_inner_helper(env: &mut Environment, arr: id, vec: Vec<id>) -> id {
    let host_object = Box::new(ObjectEnumeratorHostObject {
        array: arr,
        iterator: vec.into_iter(),
    });
    retain(env, arr);
    let class = env
        .objc
        .get_known_class("_touchHLE_NSArray_ObjectEnumerator", &mut env.mem);
    let enumerator = env.objc.alloc_object(class, host_object, &mut env.mem);
    autorelease(env, enumerator)
}
