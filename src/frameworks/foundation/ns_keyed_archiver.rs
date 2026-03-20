/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSKeyedArchiver` and serialization of its object graph format.
//!
//! Resources:
//! - You can get a good intuitive grasp of how the format works just by staring
//!   at a pretty-print of a simple archive file from something that can parse
//!   plists, e.g. `plutil -p` or `println!("{:#?}", plist::Value::...);`.
//! - Apple's [Archives and Serializations Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Archiving/Articles/archives.html)

use std::collections::HashMap;
use std::io::Cursor;

use plist::{to_writer_binary, Dictionary, Uid, Value};

use crate::frameworks::foundation::ns_keyed_unarchiver::NSKeyedArchiveRootObjectKey;
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;

struct NSKeyedArchiverHostObject {
    plist: Dictionary,
    encoded_data: id, // NSData *
    current_key: Option<Uid>,
    /// map of id => Uid
    already_archived: HashMap<id, Uid>,
}
impl HostObject for NSKeyedArchiverHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSKeyedArchiver: NSCoder

+ (id)allocWithZone:(NSZonePtr)_zone {
    let mut plist = Dictionary::new();
    // Archives made by NSKeyedArchiver have the plist pre-populated with these
    plist.insert("$archiver".into(), "NSKeyedArchiver".into());
    // The $objects begins with nil, stored as the string "$null".
    plist.insert("$objects".into(), Value::Array(vec![Value::String("$null".into())]));
    plist.insert("$top".into(), Dictionary::new().into());
    plist.insert("$version".into(), 100000.into());
    // Map nil to the first element in the $objects array ("$null")
    let mut already_archived = HashMap::new();
    already_archived.insert(nil, Uid::new(0));
    env.objc.alloc_object(this, Box::new(NSKeyedArchiverHostObject {
        plist,
        encoded_data: nil,
        current_key: None,
        already_archived
    }), &mut env.mem)
}

+ (id)archivedDataWithRootObject:(id)root_object { // NSCoding *
    let key = get_static_str(env, NSKeyedArchiveRootObjectKey);
    let instance: id = msg_class![env; NSKeyedArchiver new];
    () = msg![env; instance encodeObject:root_object forKey:key];
    let data: id = msg![env; instance encodedData];
    let data: id = msg_class![env; NSData dataWithData:data];
    release(env, instance);
    data
}

+ (bool)archiveRootObject:(id)root_object // NSCoding *
                   toFile:(id)file { // NSString *
    log_dbg!("[NSKeyedArchiver archiveRootObject:{:?} toFile:{:?}('{}')]", root_object, file, to_rust_string(env, file));
    let data: id = msg![env; this archivedDataWithRootObject:root_object];
    msg![env; data writeToFile:file atomically:true]
}

- (())encodeObject:(id)object // NSCoding *
            forKey:(id)key { // NSString *
    let key = normalize_key(env, key);
    encode_object_for_key(env, this, object, key);
}

- (())encodeBytes:(ConstPtr<u8>)bytes
           length:(NSUInteger)length
           forKey:(id)key { // NSString *
    let key = normalize_key(env, key);
    let data = env.mem.bytes_at(bytes.cast(), length).to_vec();
    let scope = get_value_to_encode_for_current_key(env, this);
    assert!(!scope.contains_key(&key));
    scope.insert(key, Value::Data(data));
}

- (())finishEncoding {
    let plist = &env.objc.borrow::<NSKeyedArchiverHostObject>(this).plist;
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    to_writer_binary(cursor, plist).unwrap();
    let len = buffer.len() as GuestUSize;
    let guest_buffer = env.mem.alloc(len);
    env.mem.bytes_at_mut(guest_buffer.cast(), len).copy_from_slice(&buffer[..]);
    let encoded_data: id = msg_class![env; NSData dataWithBytesNoCopy:guest_buffer length:len];
    env.objc.borrow_mut::<NSKeyedArchiverHostObject>(this).encoded_data = encoded_data;
    retain(env, encoded_data);
}

- (id)encodedData {
    if env.objc.borrow::<NSKeyedArchiverHostObject>(this).encoded_data == nil {
        () = msg![env; this finishEncoding];
    }
    env.objc.borrow::<NSKeyedArchiverHostObject>(this).encoded_data
}

- (())dealloc {
    let NSKeyedArchiverHostObject { encoded_data, .. } = *env.objc.borrow::<NSKeyedArchiverHostObject>(this);
    release(env, encoded_data);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

fn normalize_key(env: &mut Environment, key: id) -> String {
    assert_ne!(key, nil);
    let key = to_rust_string(env, key);
    assert!(!key.starts_with('$')); // TODO: Mangle keys with $ prefix
    key.to_string()
}

fn get_value_to_encode_for_current_key(env: &mut Environment, archiver: id) -> &mut Dictionary {
    assert_eq!(
        env.objc
            .borrow::<NSKeyedArchiverHostObject>(archiver)
            .encoded_data,
        nil
    );
    let host_object = env.objc.borrow_mut::<NSKeyedArchiverHostObject>(archiver);
    match host_object.current_key {
        Some(uid) => host_object
            .plist
            .get_mut("$objects")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .get_mut(uid.get() as usize)
            .unwrap(),
        None => host_object.plist.get_mut("$top").unwrap(),
    }
    .as_dictionary_mut()
    .unwrap()
}

fn encode_object(env: &mut Environment, archiver: id, object: id) -> Uid {
    let class = msg![env; object class];
    let host_object = env.objc.borrow_mut::<NSKeyedArchiverHostObject>(archiver);
    if let Some(existing_uid) = host_object.already_archived.get(&object).cloned() {
        // Object has already been archived, just insert a UID reference
        existing_uid
    } else {
        // Object has not been archived yet, encode it and insert reference
        host_object
            .plist
            .get_mut("$objects")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(Dictionary::new().into());
        let len = host_object.plist["$objects"].as_array().unwrap().len();
        let new_uid = Uid::new(len as u64 - 1);
        if object == class {
            // If the class selector returns itself, we're encoding a Class
            let classname = Value::String(env.objc.get_class_name(class).into());
            let mut classes = Vec::new();
            let mut current_class = class;
            while current_class != nil {
                let class_name = env.objc.get_class_name(current_class);
                classes.push(Value::String(class_name.into()));
                current_class = env.objc.get_superclass(current_class);
            }
            let host_object = env.objc.borrow_mut::<NSKeyedArchiverHostObject>(archiver);
            let entry = host_object
                .plist
                .get_mut("$objects")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .get_mut(new_uid.get() as usize)
                .unwrap()
                .as_dictionary_mut()
                .unwrap();
            entry.insert("$classes".into(), Value::Array(classes));
            entry.insert("$classname".into(), classname);
        } else {
            let previous_key = env
                .objc
                .borrow_mut::<NSKeyedArchiverHostObject>(archiver)
                .current_key
                .replace(new_uid);
            let class: id = msg![env; object class];
            encode_object_for_key(env, archiver, class, "$class".into());
            () = msg![env; object encodeWithCoder:archiver];
            env.objc
                .borrow_mut::<NSKeyedArchiverHostObject>(archiver)
                .current_key = previous_key;
        }
        new_uid
    }
}

fn encode_object_for_key(env: &mut Environment, archiver: id, object: id, normalized_key: String) {
    let uid = encode_object(env, archiver, object);
    let scope = get_value_to_encode_for_current_key(env, archiver);
    assert!(!scope.contains_key(&normalized_key));
    scope.insert(normalized_key, Value::Uid(uid));
}
