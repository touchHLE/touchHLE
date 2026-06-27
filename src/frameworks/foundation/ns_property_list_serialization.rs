/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSPropertyListSerialization`.

use super::{ns_array, ns_data, ns_dictionary, ns_string, NSInteger, NSUInteger};
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
use std::time::SystemTime;

pub type NSPropertyListMutabilityOptions = NSUInteger;
pub const NSPropertyListImmutable: NSPropertyListMutabilityOptions = 0;
pub const NSPropertyListMutableContainers: NSPropertyListMutabilityOptions = 1;
pub const NSPropertyListMutableContainersAndLeaves: NSPropertyListMutabilityOptions = 2;

pub type NSPropertyListFormat = NSUInteger;
pub const NSPropertyListXMLFormat_v1_0: NSPropertyListFormat = 100;
pub const NSPropertyListBinaryFormat_v1_0: NSPropertyListFormat = 200;

/// Options for reading a property list. The numeric values are identical to
/// the legacy `NSPropertyListMutabilityOptions`, which is why Apple lets the
/// two be used interchangeably.
pub type NSPropertyListReadOptions = NSUInteger;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSPropertyListSerialization: NSObject

+ (id)dataFromPropertyList:(id)plist
                    format:(NSPropertyListFormat)_format
                errorDescription:(MutPtr<id>)_error_string { // NSString **
    // assert_eq!(format, NSPropertyListBinaryFormat_v1_0); // TODO
    // assert!(error_string.is_null()); // TODO

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

// `+ (id)propertyListWithData:(NSData *)data
//             options:(NSPropertyListReadOptions)opt
//              format:(NSPropertyListFormat *)format
//               error:(NSError **)error;`
//
// Modern replacement (available since iOS 4.0 / macOS 10.6) for the
// deprecated `propertyListFromData:mutabilityOption:format:errorDescription:`.
// Per Apple's Foundation documentation
// (<https://developer.apple.com/documentation/foundation/nspropertylistserialization/1409678-propertylist>):
// `opt` selects container/leaf mutability (same numeric values as the legacy
// mutability options), `format` (which may be NULL) receives the detected
// on-disk format, `error` (which may be NULL) receives an `NSError` describing
// any failure, and the method returns the decoded, autoreleased property-list
// object — or `nil` when the data cannot be parsed.
+ (id)propertyListWithData:(id)data // NSData *
                   options:(NSPropertyListReadOptions)opt
                    format:(MutPtr<NSPropertyListFormat>)format
                     error:(MutPtr<id>)error { // NSError **
    if data == nil {
        if !error.is_null() {
            let err = make_plist_read_error(env, "Data parameter is nil");
            env.mem.write(error, err);
        }
        return nil;
    }

    let slice = ns_data::to_rust_slice(env, data);

    // Apple's parser auto-detects the format. Try XML first, then binary,
    // matching the legacy code path above.
    let parsed = Value::from_reader_xml(Cursor::new(slice))
        .map(|root| (root, NSPropertyListXMLFormat_v1_0))
        .or_else(|_| {
            Value::from_reader(Cursor::new(slice))
                .map(|root| (root, NSPropertyListBinaryFormat_v1_0))
        });

    match parsed {
        Ok((root, detected_format)) => {
            // A well-formed property list's root object is always a container
            // (dictionary or array). Anything else is treated as corrupt.
            if root.as_array().is_none() && root.as_dictionary().is_none() {
                if !error.is_null() {
                    let err = make_plist_read_error(
                        env,
                        "Property list root is neither a dictionary nor an array",
                    );
                    env.mem.write(error, err);
                }
                return nil;
            }
            if !format.is_null() {
                env.mem.write(format, detected_format);
            }
            if !error.is_null() {
                env.mem.write(error, nil);
            }
            let property_list = deserialize_plist(env, &root, opt);
            autorelease(env, property_list)
        }
        Err(_) => {
            if !error.is_null() {
                let err = make_plist_read_error(env, "Failed to parse property list data");
                env.mem.write(error, err);
            }
            nil
        }
    }
}

@end

};

/// Build an autoreleased `NSError` in the Cocoa error domain describing a
/// property-list read failure. The code `3840` is
/// `NSPropertyListReadCorruptError`, the value Foundation reports when the
/// supplied data is not a valid property list. Returned by
/// `+propertyListWithData:options:format:error:` on failure.
fn make_plist_read_error(env: &mut Environment, message: &str) -> id {
    log_dbg!("NSPropertyListSerialization read error: {}", message);
    let domain = ns_string::from_rust_string(env, String::from("NSCocoaErrorDomain"));
    let code: NSInteger = 3840; // NSPropertyListReadCorruptError
    let err: id = msg_class![env; NSError errorWithDomain:domain code:code userInfo:nil];
    release(env, domain);
    err
}

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
                _ => {
                    log!(
                        "Warning: deserialize_plist(array): unknown mutability option {}; treating as immutable.",
                        mut_options
                    );
                    ns_array::from_vec(env, array)
                }
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
                _ => {
                    log!(
                        "Warning: deserialize_plist(dict): unknown mutability option {}; treating as immutable.",
                        mut_options
                    );
                    ns_dictionary::dict_from_keys_and_objects(env, &pairs)
                }
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
                _ => {
                    log!(
                        "Warning: deserialize_plist(data): unknown mutability option {}; treating as immutable NSData.",
                        mut_options
                    );
                    msg_class![env; NSData alloc]
                }
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
                // plist crate docs say this is unreachable, but if it ever
                // happens, return a 0-valued NSNumber instead of panicking.
                log!("Warning: deserialize_plist: integer with no signed/unsigned repr; returning 0.");
                msg![env; number initWithLongLong:(0_i64)]
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
            _ => {
                log!(
                    "Warning: deserialize_plist: unknown mutability option {}; \
                     treating as immutable.",
                    mut_options
                );
                ns_string::from_rust_string(env, s.clone())
            }
        },
        Value::Uid(_) => {
            // These are produced by NSKeyedUnarchiver. The plist-level
            // deserializer doesn't know how to resolve them on its own;
            // return nil so callers can detect and handle the case rather
            // than crashing the whole emulator.
            log!(
                "Warning: deserialize_plist: encountered NSKeyedArchiver UID outside \
                 of NSKeyedUnarchiver; returning nil."
            );
            nil
        }
        _ => {
            log!(
                "Warning: deserialize_plist: unknown plist value {:?}; returning nil.",
                value
            );
            nil
        }
    }
}

fn serialize_plist(env: &mut Environment, plist: id) -> Value {
    let class: Class = msg![env; plist class];

    let dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    let arr_class = env.objc.get_known_class("NSArray", &mut env.mem);
    let str_class = env.objc.get_known_class("NSString", &mut env.mem);
    let num_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    let data_class = env.objc.get_known_class("NSData", &mut env.mem);
    let date_class = env.objc.get_known_class("NSDate", &mut env.mem);

    if env.objc.class_is_subclass_of(class, dict_class) {
        if !env.objc.get_class_name(class).starts_with("_touchHLE_NS") {
            log!(
                "Warning: serialize_plist: dictionary subclass {} is not our \
                 internal implementation; serializing as empty dict.",
                env.objc.get_class_name(class)
            );
            return Value::Dictionary(plist::dictionary::Dictionary::new());
        }

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
            if !env.objc.class_is_subclass_of(key_class, str_class)
                || !env
                    .objc
                    .get_class_name(key_class)
                    .starts_with("_touchHLE_NS")
            {
                log!(
                    "Warning: serialize_plist: dropping non-string or external \
                     dictionary key (class {}).",
                    env.objc.get_class_name(key_class)
                );
                continue;
            }

            let key_string = ns_string::to_rust_string(env, key);
            let val_plist = serialize_plist(env, val);
            dict.insert(String::from(key_string), val_plist);
        }
        Value::Dictionary(dict)
    } else if env.objc.class_is_subclass_of(class, arr_class) {
        if !env.objc.get_class_name(class).starts_with("_touchHLE_NS") {
            log!(
                "Warning: serialize_plist: array subclass {} is not our internal \
                 implementation; serializing as empty array.",
                env.objc.get_class_name(class)
            );
            return Value::Array(Vec::new());
        }

        let arr_host_obj: ArrayHostObject = std::mem::take(env.objc.borrow_mut(plist));
        let arr: Vec<Value> = arr_host_obj
            .array
            .iter()
            .map(|&value| serialize_plist(env, value))
            .collect();
        *env.objc.borrow_mut(plist) = arr_host_obj;
        Value::Array(arr)
    } else if env.objc.class_is_subclass_of(class, str_class) {
        if !env.objc.get_class_name(class).starts_with("_touchHLE_NS") {
            log!(
                "Warning: serialize_plist: string subclass {} is not our internal \
                 implementation; serializing as empty string.",
                env.objc.get_class_name(class)
            );
            return Value::String(String::new());
        }

        let s = ns_string::to_rust_string(env, plist);
        Value::String(s.to_string())
    } else if env.objc.class_is_subclass_of(class, num_class) {
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
            NSNumberHostObject::UnsignedLongLong(ull) => Value::from(*ull),
            NSNumberHostObject::UnsignedShort(us) => Value::from(*us),
        }
    } else if env.objc.class_is_subclass_of(class, data_class) {
        let data = env.objc.borrow::<NSDataHostObject>(plist);
        let buffer_slice = env.mem.bytes_at(data.bytes.cast(), data.length);
        Value::Data(buffer_slice.to_vec())
    } else if env.objc.class_is_subclass_of(class, date_class) {
        let date = env.objc.borrow::<NSDateHostObject>(plist);
        let time = apple_epoch().add(
            crate::frameworks::foundation::ns_time_interval_to_duration_or_zero(date.time_interval),
        );
        Value::Date(time.into())
    } else {
        warn_unsupported_serialize_class_once(env.objc.get_class_name(class));
        // Per Apple's NSPropertyListSerialization docs, only NSData / NSDate /
        // NSNumber / NSString / NSArray / NSDictionary are encodable. Anything
        // else *should* raise NSInvalidArgumentException, but we want to keep
        // the archive valid for debugging purposes. Fall back to whatever
        // -[<plist> description] returns —
        let description = msg![env; plist description];
        Value::String(ns_string::to_rust_string(env, description).to_string())
    }
}

fn warn_unsupported_serialize_class_once(class_name: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = seen.lock().unwrap();
    if guard.insert(class_name.to_string()) {
        log!(
            "Warning: serialize_plist: unsupported class {} — falling back to \
             -[<obj> description] (per Apple docs only NSData / NSDate / \
             NSNumber / NSString / NSArray / NSDictionary are plist-encodable; \
             further occurrences of this class will be silenced)",
            class_name
        );
    }
}
