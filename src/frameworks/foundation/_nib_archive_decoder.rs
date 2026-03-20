/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! UIKit NIB Archives decoder. This is _not_ a part of public API!
//!
//! Resources:
//! - [UIKit NIB Archives](https://www.mothersruin.com/software/Archaeology/reverse/uinib.html)
//! - [NibArchive File Format](https://github.com/matsmattsson/nibsqueeze/blob/master/NibArchive.md)

use crate::environment::Environment;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::mem::ConstVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use nibarchive::{NIBArchive, Value, ValueVariant};

struct NIBArchiveDecoderHostObject {
    archive: NIBArchive,
    current_object_idx: Option<u32>,
    /// linear map of idx => id
    already_unarchived: Vec<Option<id>>,
    delegate: id,
}
impl HostObject for NIBArchiveDecoderHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_NIBArchiveDecoder: NSCoder

+ (id)allocWithZone:(NSZonePtr)_zone { // struct _NSZone*
    let unarchiver = Box::new(NIBArchiveDecoderHostObject {
        archive: NIBArchive::new_unchecked(
            Default::default(), Default::default(), Default::default(), Default::default()
        ),
        current_object_idx: None,
        already_unarchived: Vec::new(),
        delegate: nil,
    });
    env.objc.alloc_object(this, unarchiver, &mut env.mem)
}

- (id)_touchHLE_initForReadingWithData:(id)data {
    if data == nil {
        return nil;
    }

    let length: NSUInteger = msg![env; data length];
    let bytes: ConstVoidPtr = msg![env; data bytes];

    let archive: NIBArchive = NIBArchive::from_bytes(env.mem.bytes_at(bytes.cast(), length)).unwrap();

    // for (i, object) in archive.objects().iter().enumerate() {
    //     let class_name = object.class_name(archive.class_names()).name();
    //     println!("[{i}] Object of a class '{class_name}':");
    //
    //     let values: &[nibarchive::Value] = object.values(archive.values());
    //     for (j, value) in values.iter().enumerate() {
    //         let key = value.key(archive.keys());
    //         let inner_value = value.value();
    //         println!("-- [{j}] {key}: {inner_value:?}");
    //     }
    // }

    let host_obj = env.objc.borrow_mut::<NIBArchiveDecoderHostObject>(this);
    assert!(host_obj.already_unarchived.is_empty());
    assert!(host_obj.current_object_idx.is_none());

    let objects_count = archive.objects().len();

    host_obj.already_unarchived = vec![None; objects_count];
    host_obj.archive = archive;

    // we start from the 'top'
    host_obj.current_object_idx = Some(0);

    this
}

- (())dealloc {
    let host_obj: &mut NIBArchiveDecoderHostObject = env.objc.borrow_mut(this);
    let already_unarchived = std::mem::take(&mut host_obj.already_unarchived);

    for &object in already_unarchived.iter().flatten() {
        release(env, object);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: implement calls to delegate methods
// weak/non-retaining
- (())setDelegate:(id)delegate { // id<?>
    let host_object = env.objc.borrow_mut::<NIBArchiveDecoderHostObject>(this);
    host_object.delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<NIBArchiveDecoderHostObject>(this).delegate
}

// These methods drive most of the decoding. They get called in two cases:
// - By the code that initiates the unarchival, e.g. UINib, to retrieve
//   top-level objects.
// - By the object currently being unarchived, i.e. something that had
//   `initWithCoder:` called on it, to retrieve objects from its scope.
// They are all from the NSCoder abstract class and they return default values
// if the key is unknown.

- (bool)decodeBoolForKey:(id)key { // NSString *
    get_value_to_decode_for_key(env, this, key).is_some_and(|value| {
        let &ValueVariant::Bool(b) = value.value() else {
            unreachable!()
        };
        b
    })
}

- (f32)decodeFloatForKey:(id)key { // NSString *
    // TODO: Check bounds, raise NSRangeException if it doesn't fit
    get_value_to_decode_for_key(env, this, key).map_or(
        0.0,
        |value| {
            let &ValueVariant::Float(f) = value.value() else {
                unreachable!()
            };
            f
        }
    )
}

- (NSInteger)decodeIntegerForKey:(id)key { // NSString *
    // TODO: Check bounds, raise NSRangeException if it doesn't fit
    get_value_to_decode_for_key(env, this, key).map_or(
        0,
        |value| {
            match value.value() {
                ValueVariant::Int8(i) => (*i).into(),
                ValueVariant::Int16(i) => (*i).into(),
                ValueVariant::Int32(i) => *i,
                ValueVariant::Int64(i) => (*i).try_into().unwrap(),
                _ => unreachable!() // Not sure what should happen here?
            }
        }
    )
}
- (i32)decodeIntForKey:(id)key { // NSString *
    msg![env; this decodeIntegerForKey:key]
}


- (id)decodeObjectForKey:(id)key { // NSString*
    let Some(val) = get_value_to_decode_for_key(env, this, key) else {
        return nil;
    };
    let &ValueVariant::ObjectRef(idx) = val.value() else {
        unreachable!()
    };

    let object = unarchive_obj(env, this, idx);

    // on behalf of the caller
    retain(env, object);
    autorelease(env, object)
}

- (bool)containsValueForKey:(id)key { // NSString*
    assert!(key != nil);
    get_value_to_decode_for_key(env, this, key).is_some()
}

// These come from a category in UIKit's UIGeometry.h
- (CGPoint)decodeCGPointForKey:(id)key { // NSString*
    let val = get_value_to_decode_for_key(env, this, key).unwrap();
    let ValueVariant::Data(data) = val.value() else {
        unreachable!()
    };
    assert_eq!(6, data[0]);
    let x = f32::from_le_bytes(data[1..5].try_into().unwrap());
    let y = f32::from_le_bytes(data[5..9].try_into().unwrap());
    log_dbg!("decoded CGPoint {} {}", x, y);
    CGPoint { x, y }
}
- (CGRect)decodeCGRectForKey:(id)key { // NSString*
    let val = get_value_to_decode_for_key(env, this, key).unwrap();
    let ValueVariant::Data(data) = val.value() else {
        unreachable!()
    };
    assert_eq!(6, data[0]);
    let x = f32::from_le_bytes(data[1..5].try_into().unwrap());
    let y = f32::from_le_bytes(data[5..9].try_into().unwrap());
    let width = f32::from_le_bytes(data[9..13].try_into().unwrap());
    let height = f32::from_le_bytes(data[13..17].try_into().unwrap());
    log_dbg!("decoded CGRect {} {} {} {}", x, y, width, height);
    CGRect {
        origin: CGPoint { x, y },
        size: CGSize { width, height },
    }
}

@end

};

fn borrow_host_obj(env: &mut Environment, unarchiver: id) -> &mut NIBArchiveDecoderHostObject {
    env.objc.borrow_mut(unarchiver)
}

fn get_value_to_decode_for_key(env: &mut Environment, unarchiver: id, key: id) -> Option<&Value> {
    let key = to_rust_string(env, key); // TODO: avoid copying string
    let host_obj = borrow_host_obj(env, unarchiver);
    let current_idx = host_obj.current_object_idx.unwrap();

    let obj = host_obj
        .archive
        .objects()
        .get(current_idx as usize)
        .unwrap();
    obj.values(host_obj.archive.values())
        .iter()
        .find(|&val| val.key(host_obj.archive.keys()) == &key)
        .map(|v| v as _)
}

fn unarchive_obj(env: &mut Environment, unarchiver: id, idx: u32) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    if let Some(existing) = host_obj.already_unarchived[idx as usize] {
        return existing;
    }

    let object = host_obj.archive.objects().get(idx as usize).unwrap();
    let class_name = object.class_name(host_obj.archive.class_names()).name();

    log_dbg!(
        "Unarchiving object of a class '{}' at index {}",
        class_name,
        idx
    );

    let class = {
        // get_known_class needs &mut ObjC, so we can't call it
        // while holding a reference to the class name, since it
        // is ultimately owned by ObjC via the host object
        let class_name = class_name.to_string();
        env.objc.get_known_class(&class_name, &mut env.mem)
    };

    let host_obj = borrow_host_obj(env, unarchiver); // reborrow
    let old_current_idx = host_obj.current_object_idx;
    host_obj.current_object_idx = Some(idx);

    let new_object: id = msg![env; class alloc];
    let new_object: id = msg![env; new_object initWithCoder:unarchiver];

    let host_obj = borrow_host_obj(env, unarchiver); // reborrow
    host_obj.current_object_idx = old_current_idx;

    let host_obj = borrow_host_obj(env, unarchiver); // reborrow
    host_obj.already_unarchived[idx as usize] = Some(new_object);
    new_object
}

pub fn decode_current_array(env: &mut Environment, unarchiver: id) -> Vec<id> {
    let host_obj = borrow_host_obj(env, unarchiver);
    let current_idx = host_obj.current_object_idx.unwrap();

    let mut indicies = vec![];

    let object = host_obj
        .archive
        .objects()
        .get(current_idx as usize)
        .unwrap();
    let values: &[Value] = object.values(host_obj.archive.values());
    for (i, value) in values.iter().enumerate() {
        let key = value.key(host_obj.archive.keys());
        let inner_value = value.value();
        if i == 0 {
            assert_eq!("NSInlinedValue", key);
            let &ValueVariant::Bool(b) = inner_value else {
                unreachable!()
            };
            assert!(b); //  Can this ever be false?
            continue;
        }
        assert_eq!("UINibEncoderEmptyKey", key);

        let &ValueVariant::ObjectRef(next_idx) = inner_value else {
            unreachable!()
        };
        indicies.push(next_idx);
    }

    let mut array: Vec<id> = vec![];
    for next_idx in indicies {
        let host_obj = borrow_host_obj(env, unarchiver); // reborrow
        let old_current_idx = host_obj.current_object_idx;
        host_obj.current_object_idx = Some(next_idx);

        let next_obj = unarchive_obj(env, unarchiver, next_idx);
        retain(env, next_obj);
        array.push(next_obj);

        let host_obj = borrow_host_obj(env, unarchiver); // reborrow
        host_obj.current_object_idx = old_current_idx;
    }
    array
}

pub fn decode_current_dict(env: &mut Environment, unarchiver: id) -> Vec<(id, id)> {
    // Trick: dict is an array of key-value-key-value sequence
    let array = decode_current_array(env, unarchiver);
    // Note: we need to release as `dict_from_keys_and_objects`
    // would retain them later
    for &obj in &array {
        release(env, obj);
    }
    array
        .chunks_exact(2)
        .map(|chunks| (chunks[0], chunks[1]))
        .collect()
}

pub fn decode_current_string(env: &mut Environment, unarchiver: id) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    let current_idx = host_obj.current_object_idx.unwrap();

    let object = host_obj
        .archive
        .objects()
        .get(current_idx as usize)
        .unwrap();
    let values: &[Value] = object.values(host_obj.archive.values());
    assert_eq!(1, values.len());

    let value = &values[0];
    let key = value.key(host_obj.archive.keys());
    assert_eq!("NS.bytes", key);

    let inner_value = value.value();
    let ValueVariant::Data(inner_data) = inner_value else {
        unreachable!()
    };

    let str = String::from_utf8(inner_data.clone()).unwrap();
    log_dbg!("decode_current_string {str}");
    from_rust_string(env, str)
}

pub fn decode_current_number(env: &mut Environment, unarchiver: id) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    let current_idx = host_obj.current_object_idx.unwrap();

    let object = host_obj
        .archive
        .objects()
        .get(current_idx as usize)
        .unwrap();
    let values: &[Value] = object.values(host_obj.archive.values());
    assert_eq!(1, values.len());

    let value = &values[0];
    let key = value.key(host_obj.archive.keys());

    match key.as_str() {
        "NS.intval" => {
            let ns_key: id = get_static_str(env, "NS.intval");
            let int: NSInteger = msg![env; unarchiver decodeIntegerForKey:ns_key];
            release(env, ns_key);
            let num = msg_class![env; NSNumber alloc];
            msg![env; num initWithInteger:int]
        }
        _ => unimplemented!("decode_current_number: {key}"),
    }
}
