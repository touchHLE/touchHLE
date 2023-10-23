//! `NSPropertyListSerialization`.

use super::{ns_array, ns_dictionary, ns_string, NSUInteger};
use crate::frameworks::foundation::ns_array::to_vec;
use crate::frameworks::foundation::ns_data::to_rust_slice;
use crate::frameworks::foundation::ns_dictionary::dict_to_keys_and_objects;
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::objc::{id, msg, msg_class, nil, release, ClassExports};
use crate::{objc_classes, Environment};
use plist::{Dictionary, Integer, Value};
use std::io::Cursor;

// TODO: Implement reading of property lists other than Info.plist.
// [NSDictionary contentsOfFile:] and [NSArray contentsOfFile:] in particular.

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

    let Ok(root) = Value::from_reader(Cursor::new(bytes)) else {
        log_dbg!("Couldn't parse plist, returning nil.");
        return nil;
    };

    if array_expected && root.as_array().is_none() {
        log_dbg!("Plist root is not array, returning nil.");
        return nil;
    }
    if !array_expected && root.as_dictionary().is_none() {
        log_dbg!("Plist root is not dictionary, returning nil.");
        return nil;
    }

    deserialize_plist(env, &root)
}

fn deserialize_plist(env: &mut Environment, value: &Value) -> id {
    match value {
        Value::Array(array) => {
            let array = array
                .iter()
                .map(|value| deserialize_plist(env, value))
                .collect();
            ns_array::from_vec(env, array)
        }
        Value::Dictionary(dict) => {
            let pairs: Vec<_> = dict
                .iter()
                .map(|(key, value)| {
                    (
                        ns_string::from_rust_string(env, key.clone()),
                        deserialize_plist(env, value),
                    )
                })
                .collect();
            // Unlike ns_array::from_vec and ns_string::from_rust_string,
            // this will retain the keys and values!
            let ns_dict = ns_dictionary::dict_from_keys_and_objects(env, &pairs);
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
            let alloc: MutPtr<u8> = env.mem.alloc(length).cast();
            env.mem.bytes_at_mut(alloc, length).copy_from_slice(d);
            let data: id = msg_class![env; NSMutableData alloc];
            msg![env; data initWithBytesNoCopy:(alloc.cast_void()) length:length]
        }
        Value::Date(_) => {
            todo!("deserialize plist value: {:?}", value); // TODO
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
        Value::String(s) => ns_string::from_rust_string(env, s.clone()),
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

fn serialize_plist(env: &mut Environment, obj: id) -> Value {
    let class: id = msg![env; obj class];
    let dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    let number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    let string_class = env.objc.get_known_class("NSString", &mut env.mem);
    let data_class = env.objc.get_known_class("NSData", &mut env.mem);
    let array_class = env.objc.get_known_class("NSArray", &mut env.mem);
    if msg![env; obj isKindOfClass: dict_class] {
        let mut pdict = Dictionary::new();
        for (k, v) in dict_to_keys_and_objects(env, obj) {
            pdict.insert(to_rust_string(env, k).to_string(), serialize_plist(env, v));
        }
        Value::Dictionary(pdict)
    } else if msg![env; obj isKindOfClass: number_class] {
        let type_str: ConstPtr<u8> = msg![env; obj objCType];
        match env.mem.read(type_str) {
            b'B' => Value::Boolean(msg![env; obj boolValue]),
            b'Q' => {
                let val: u64 = msg![env; obj unsignedLongLongValue];
                Value::Integer(Integer::from(val))
            }
            b'q' => {
                let val: i64 = msg![env; obj longLongValue];
                Value::Integer(Integer::from(val))
            }
            b'd' => Value::Real(msg![env; obj doubleValue]),
            t => todo!("Unknown type: {}", t),
        }
    } else if msg![env; obj isKindOfClass: string_class] {
        Value::String(to_rust_string(env, obj).to_string())
    } else if msg![env; obj isKindOfClass: data_class] {
        Value::Data(to_rust_slice(env, obj).to_vec())
    } else if msg![env; obj isKindOfClass: array_class] {
        Value::Array(
            to_vec(env, obj)
                .iter()
                .map(|&x| serialize_plist(env, x))
                .collect(),
        )
    } else {
        todo!(
            "Serializing {} not supported yet",
            env.objc.get_class_name(class)
        )
    }
}

pub type NSPropertyListFormat = NSUInteger;
pub const NSPropertyListXMLFormat_v1_0: NSPropertyListFormat = 100;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSPropertyListSerialization: NSObject

+ (id)dataFromPropertyList:(id)plist
                    format:(NSPropertyListFormat)format
          errorDescription:(MutPtr<id>)error {
    let val = serialize_plist(env, plist);
    let mut data = Vec::new();
    match format {
        NSPropertyListXMLFormat_v1_0 => val.to_writer_xml(&mut data).unwrap(),
        f => todo!("Unimplemented plist serialization format: {}", f),
    };
    let len = data.len() as GuestUSize;
    let ptr = env.mem.alloc(len).cast();
    env.mem.bytes_at_mut(ptr, len).copy_from_slice(&data);
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    msg_class![env; NSData dataWithBytesNoCopy:(ptr.cast_void())
                                        length:len]
}

+ (id)propertyListFromData:(id)data
          mutabilityOption:(NSUInteger)opt
                    format:(MutPtr<NSPropertyListFormat>)format
          errorDescription:(MutPtr<id>)err {
    assert_eq!(opt, 2);
    if !err.is_null() {
        env.mem.write(err, nil);
    }
    if !format.is_null() {
        env.mem.write(format, NSPropertyListXMLFormat_v1_0);
    }
    let bytes = to_rust_slice(env, data);
    let Ok(root) = Value::from_reader(Cursor::new(bytes)) else {
        log_dbg!("Couldn't parse plist, returning nil.");
        return nil;
    };
    deserialize_plist(env, &root)
}

@end

};
