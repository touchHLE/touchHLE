//! `NSPropertyListSerialization`.

use super::{ns_array, ns_dictionary, ns_string, NSUInteger};
use crate::fs::GuestPath;
use crate::mem::MutPtr;
use crate::objc::{id, msg, msg_class, nil, release};
use crate::Environment;
use plist::Value;
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
            let data: id = msg_class![env; NSData alloc];
            msg![env; data initWithBytesNoCopy:alloc length:length]
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
