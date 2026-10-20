/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSPropertyListSerialization`.

use super::{ns_array, ns_data, ns_dictionary, ns_string, NSUInteger};
use super::{
    ns_array::ArrayHostObject, ns_data::NSDataHostObject, ns_dictionary::DictionaryHostObject,
    ns_value::NSNumberHostObject,
};
use crate::frameworks::core_foundation::time::apple_epoch;
use crate::frameworks::foundation::ns_date::NSDateHostObject;
use crate::fs::GuestPath;
use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, Class, ClassExports,
};
use crate::Environment;
use plist::Value;
use std::io::Cursor;
use std::ops::Add;
use std::time::{Duration, SystemTime};

pub type NSPropertyListMutabilityOptions = NSUInteger;
pub const NSPropertyListImmutable: NSPropertyListMutabilityOptions = 0;
pub const NSPropertyListMutableContainers: NSPropertyListMutabilityOptions = 1;
pub const NSPropertyListMutableContainersAndLeaves: NSPropertyListMutabilityOptions = 2;

pub type NSPropertyListFormat = NSUInteger;
pub const NSPropertyListXMLFormat_v1_0: NSPropertyListFormat = 100;
pub const NSPropertyListBinaryFormat_v1_0: NSPropertyListFormat = 200;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSPropertyListSerialization: NSObject

+ (id)dataFromPropertyList:(id)plist
                    format:(NSPropertyListFormat)format
                errorDescription:(MutPtr<id>)error_string { // NSString **
    assert_eq!(format, NSPropertyListBinaryFormat_v1_0); // TODO
    assert!(error_string.is_null()); // TODO

    let value = serialize_plist(env, plist);
    log_dbg!("dataFromPropertyList value {:?}", value);
    let mut buf = Vec::new();
    value.to_writer_binary(&mut buf).unwrap();
    let len: u32 = buf.len().try_into().unwrap();
    log_dbg!("dataFromPropertyList buf len {}", len);
    let ptr = env.mem.alloc(len);
    env.mem.bytes_at_mut(ptr.cast(), len).copy_from_slice(&buf[..]);
    msg_class![env; NSData dataWithBytesNoCopy:ptr length:len]
}

+ (id)propertyListFromData:(id)data // NSData *
          mutabilityOption:(NSPropertyListMutabilityOptions)opt
                    format:(MutPtr<NSPropertyListFormat>)format
          errorDescription:(MutPtr<id>)error_string { // NSString **
    let slice = ns_data::to_rust_slice(env, data);

    if let Ok(root) = Value::from_reader_xml(Cursor::new(slice)) {
        assert!(root.as_array().is_some() || root.as_dictionary().is_some());
        if !format.is_null() {
            env.mem.write(format, NSPropertyListXMLFormat_v1_0);
        }
        let property_list = deserialize_plist(env, &root, opt);
        return autorelease(env, property_list)
    }

    if let Ok(root) = Value::from_reader(Cursor::new(slice)) {
        assert!(root.as_array().is_some() || root.as_dictionary().is_some());
        if !format.is_null() {
            env.mem.write(format, NSPropertyListBinaryFormat_v1_0);
        }
        let property_list = deserialize_plist(env, &root, opt);
        return autorelease(env, property_list)
    }

    if !error_string.is_null() {
        let error_message = ns_string::from_rust_string(env, String::from("Failed to parse plist"));
        env.mem.write(error_string, error_message);
        autorelease(env, error_message);
    }

    nil
}

@end

};

/// Internals of `initWithContentsOfFile:` on `NSArray` and `NSDictionary`.
/// Returns `nil` on failure.
pub(super) fn deserialize_plist_from_file(
    env: &mut Environment,
    path: &GuestPath,
    array_expected: bool,
) -> id {
    log_dbg!("Reading plist from {:?}.", path);
    let Ok(bytes) = env.fs.read(path) else {
        log_dbg!("Couldn't read file, returning nil.");
        return nil;
    };

    let root = match Value::from_reader(Cursor::new(bytes)) {
        Ok(root) => root,
        Err(err) => {
            log_dbg!("Couldn't parse plist, returning nil: {}", err);
            return nil;
        }
    };

    if array_expected && root.as_array().is_none() {
        log_dbg!("Plist root is not array, returning nil.");
        return nil;
    }
    if !array_expected && root.as_dictionary().is_none() {
        log_dbg!("Plist root is not dictionary, returning nil.");
        return nil;
    }

    // Note: The top-most container mutability may change
    // depending on the caller.
    // (see `NSMutableArray` and `NSMutableDictionary` implementations)
    deserialize_plist(env, &root, NSPropertyListImmutable)
}

fn deserialize_plist(
    env: &mut Environment,
    value: &Value,
    mut_options: NSPropertyListMutabilityOptions,
) -> id {
    match value {
        Value::Array(array) => {
            let array = array
                .iter()
                .map(|value| deserialize_plist(env, value, mut_options))
                .collect();
            match mut_options {
                NSPropertyListImmutable => ns_array::from_vec(env, array),
                NSPropertyListMutableContainers | NSPropertyListMutableContainersAndLeaves => {
                    ns_array::mutable_from_vec(env, array)
                }
                _ => unreachable!(),
            }
        }
        Value::Dictionary(dict) => {
            let pairs: Vec<_> = dict
                .iter()
                .map(|(key, value)| {
                    (
                        ns_string::from_rust_string(env, key.clone()),
                        deserialize_plist(env, value, mut_options),
                    )
                })
                .collect();
            // Unlike ns_array::from_vec and ns_string::from_rust_string,
            // this will retain the keys and values!
            let ns_dict = match mut_options {
                NSPropertyListImmutable => ns_dictionary::dict_from_keys_and_objects(env, &pairs),
                NSPropertyListMutableContainers | NSPropertyListMutableContainersAndLeaves => {
                    ns_dictionary::mutable_dict_from_keys_and_objects(env, &pairs)
                }
                _ => unreachable!(),
            };
            // ...so they need to be released.
            for (key, value) in pairs {
                release(env, key);
                release(env, value);
            }
            ns_dict
        }
        Value::Boolean(b) => {
            let number: id = msg_class![env; NSNumber alloc];
            let b: bool = *b;
            msg![env; number initWithBool:b]
        }
        Value::Data(d) => {
            let length: NSUInteger = d.len().try_into().unwrap();
            let alloc: MutVoidPtr = env.mem.alloc(length);
            env.mem
                .bytes_at_mut(alloc.cast(), length)
                .copy_from_slice(d);
            let ns_data = match mut_options {
                NSPropertyListImmutable | NSPropertyListMutableContainers => {
                    msg_class![env; NSData alloc]
                }
                NSPropertyListMutableContainersAndLeaves => msg_class![env; NSMutableData alloc],
                _ => unreachable!(),
            };
            msg![env; ns_data initWithBytesNoCopy:alloc length:length]
        }
        Value::Date(date_val) => {
            let time: SystemTime = (*date_val).into();
            let time_interval = time.duration_since(apple_epoch()).unwrap().as_secs_f64();
            let date: id = msg_class![env; NSDate alloc];
            msg![env; date initWithTimeIntervalSinceReferenceDate:time_interval]
        }
        Value::Integer(int) => {
            let number: id = msg_class![env; NSNumber alloc];
            // TODO: is this the correct order of preference? does it matter?
            if let Some(int64) = int.as_signed() {
                let longlong: i64 = int64;
                msg![env; number initWithLongLong:longlong]
            } else if let Some(uint64) = int.as_unsigned() {
                let ulonglong: u64 = uint64;
                msg![env; number initWithUnsignedLongLong:ulonglong]
            } else {
                unreachable!(); // according to plist crate docs
            }
        }
        Value::Real(real) => {
            let number: id = msg_class![env; NSNumber alloc];
            let double: f64 = *real;
            msg![env; number initWithDouble:double]
        }
        Value::String(s) => match mut_options {
            NSPropertyListImmutable | NSPropertyListMutableContainers => {
                ns_string::from_rust_string(env, s.clone())
            }
            NSPropertyListMutableContainersAndLeaves => {
                ns_string::mutable_from_rust_string(env, s.clone())
            }
            _ => unreachable!(),
        },
        Value::Uid(_) => {
            // These are probably only used by NSKeyedUnarchiver, which does not
            // currently use this code in our implementation.
            unimplemented!("deserialize plist value: {:?}", value);
        }
        _ => {
            unreachable!() // enum is marked inexhaustive, but shouldn't be
        }
    }
}

fn serialize_plist(env: &mut Environment, plist: id) -> Value {
    let class: Class = msg![env; plist class];

    let dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    let arr_class = env.objc.get_known_class("NSArray", &mut env.mem);
    let str_class = env.objc.get_known_class("NSString", &mut env.mem);

    if env.objc.class_is_subclass_of(class, dict_class) {
        // only our internal implementation is supported
        assert!(env.objc.get_class_name(class).starts_with("_touchHLE_NS"));

        let mut dict = plist::dictionary::Dictionary::new();
        let dict_host_obj: DictionaryHostObject = std::mem::take(env.objc.borrow_mut(plist));
        let mut key_vals = Vec::with_capacity(dict_host_obj.count as usize);
        for collisions in dict_host_obj.map.values() {
            for &(key, value) in collisions {
                key_vals.push((key, value));
            }
        }
        *env.objc.borrow_mut(plist) = dict_host_obj;
        for (key, val) in key_vals {
            let key_class: Class = msg![env; key class];

            // only string keys are supported
            assert!(env.objc.class_is_subclass_of(key_class, str_class));
            assert!(env
                .objc
                .get_class_name(key_class)
                .starts_with("_touchHLE_NS"));

            let key_string = ns_string::to_rust_string(env, key);
            let val_plist = serialize_plist(env, val);
            dict.insert(String::from(key_string), val_plist);
        }
        Value::Dictionary(dict)
    } else if env.objc.class_is_subclass_of(class, arr_class) {
        // only our internal implementation is supported
        assert!(env.objc.get_class_name(class).starts_with("_touchHLE_NS"));

        let arr_host_obj: ArrayHostObject = std::mem::take(env.objc.borrow_mut(plist));
        let arr: Vec<Value> = arr_host_obj
            .array
            .iter()
            .map(|&value| serialize_plist(env, value))
            .collect();
        *env.objc.borrow_mut(plist) = arr_host_obj;
        Value::Array(arr)
    } else if env.objc.class_is_subclass_of(class, str_class) {
        // only our internal implementation is supported
        assert!(env.objc.get_class_name(class).starts_with("_touchHLE_NS"));

        let s = ns_string::to_rust_string(env, plist);
        Value::String(s.to_string())
    } else if class == env.objc.get_known_class("NSNumber", &mut env.mem) {
        let num = env.objc.borrow::<NSNumberHostObject>(plist);
        match num {
            NSNumberHostObject::Bool(b) => Value::Boolean(*b),
            NSNumberHostObject::Int(i) => Value::from(*i),
            NSNumberHostObject::UnsignedInt(ui) => Value::from(*ui),
            NSNumberHostObject::Float(f) => Value::from(*f),
            NSNumberHostObject::Double(d) => Value::from(*d),
            NSNumberHostObject::LongLong(ll) => Value::from(*ll),
            NSNumberHostObject::Short(s) => Value::from(*s),
            NSNumberHostObject::Char(c) => Value::from(*c),
            _ => todo!("num {:?}", num),
        }
    } else if class == env.objc.get_known_class("NSData", &mut env.mem) {
        let data = env.objc.borrow::<NSDataHostObject>(plist);
        let buffer_slice = env.mem.bytes_at(data.bytes.cast(), data.length);
        Value::Data(buffer_slice.to_vec())
    } else if class == env.objc.get_known_class("NSDate", &mut env.mem) {
        let date = env.objc.borrow::<NSDateHostObject>(plist);
        let time = apple_epoch().add(Duration::from_secs_f64(date.time_interval));
        Value::Date(time.into())
    } else {
        unimplemented!("class {}", env.objc.get_class_name(class))
    }
}
