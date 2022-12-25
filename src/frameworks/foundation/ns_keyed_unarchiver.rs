//! `NSKeyedUnarchiver` and deserialization of its object graph format.
//!
//! Resources:
//! - You can get a good intuitive grasp of how the format works just by staring
//!   at a pretty-print of a simple nib file from something that can parse
//!   plists, e.g. `plutil -p` or `println!("{:#?}", plist::Value::...);`.
//! - Apple's [Archives and Serializations Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Archiving/Articles/archives.html)

use crate::objc::{id, msg, objc_classes, ClassExports, HostObject};
use crate::Environment;
use plist::{Dictionary, Uid, Value};
use std::collections::HashMap;
use std::path::Path;

struct NSKeyedUnarchiverHostObject {
    plist: Dictionary,
    current_key: Option<Uid>,
    /// linear map of Uid => id
    already_unarchived: Vec<Option<id>>,
}
impl HostObject for NSKeyedUnarchiverHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSKeyedUnarchiver: NSCoder

// TODO: real init/unarchive methods. This is currently only created by the
// shortcut method below.

@end

};

/// Shortcut for use by [crate::frameworks::uikit::ui_nib::load_main_nib_file].
///
/// This is probably equivalent to calling `decodeObjectForKey:` once for each
/// of the top-level keys? TODO: rework this to work that way.
pub fn unarchive_object_with_file(env: &mut Environment, path: &Path) -> HashMap<String, id> {
    let plist = Value::from_file(path).unwrap();
    let plist = plist.into_dictionary().unwrap();
    assert!(plist["$version"].as_unsigned_integer() == Some(100000));
    assert!(plist["$archiver"].as_string() == Some("NSKeyedArchiver"));

    let top_level_key_list = Vec::from_iter(
        plist["$top"]
            .as_dictionary()
            .unwrap()
            .into_iter()
            .map(|(k, v)| (k.clone(), v.as_uid().copied().unwrap())),
    );

    let key_count = plist["$objects"].as_array().unwrap().len();

    let unarchiver = Box::new(NSKeyedUnarchiverHostObject {
        plist,
        current_key: None,
        already_unarchived: vec![None; key_count],
    });
    let class = env.objc.get_known_class("NSKeyedUnarchiver", &mut env.mem);
    let unarchiver = env.objc.alloc_object(class, unarchiver, &mut env.mem);

    let mut top_level_keys = HashMap::new();
    for (key_name, key_key) in top_level_key_list {
        top_level_keys.insert(key_name, unarchive_key(env, unarchiver, key_key));
    }

    let _: () = msg![env; unarchiver release];

    top_level_keys
}

fn borrow_host_obj(env: &mut Environment, unarchiver: id) -> &mut NSKeyedUnarchiverHostObject {
    env.objc.borrow_mut(unarchiver)
}

/// The core of the implementation: recursively unarchive things.
fn unarchive_key(env: &mut Environment, unarchiver: id, key: Uid) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    if let Some(existing) = host_obj.already_unarchived[key.get() as usize] {
        return existing;
    }

    let objects = host_obj.plist["$objects"].as_array().unwrap();

    let item = &objects[key.get() as usize];
    match item {
        // The most general kind of item: a dictionary that contains the info
        // needed to invoke `initWithCoder:` on a class implementing NSCoding.
        Value::Dictionary(dict) => {
            let class_key = dict["$class"].as_uid().copied().unwrap();
            let class;
            if let Some(existing) = host_obj.already_unarchived[class_key.get() as usize] {
                class = existing;
            } else {
                let class_dict = &objects[class_key.get() as usize];
                let class_dict = class_dict.as_dictionary().unwrap();

                let class_name = class_dict["$classname"].as_string().unwrap();

                class = {
                    // get_known_class needs &mut ObjC, so we can't call it
                    // while holding a reference to the class name, since it
                    // is ultimately owned by ObjC via the host object
                    let class_name = class_name.to_string();
                    env.objc.get_known_class(&class_name, &mut env.mem)
                };
                let host_obj = borrow_host_obj(env, unarchiver); // reborrow

                host_obj.already_unarchived[class_key.get() as usize] = Some(class);
            };

            let host_obj = borrow_host_obj(env, unarchiver); // reborrow
            let old_current_key = host_obj.current_key;
            host_obj.current_key = Some(key);

            let new_object: id = msg![env; class alloc];
            let new_object: id = msg![env; new_object initWithCoder:unarchiver];

            let host_obj = borrow_host_obj(env, unarchiver); // reborrow
            host_obj.current_key = old_current_key;
            host_obj.already_unarchived[key.get() as usize] = Some(new_object);
            // TODO: do we need to increase a refcount here?
            new_object
        }
        _ => unimplemented!("Unarchive: {:#?}", item),
    }
}

/// Shortcut for use by `[_touchHLE_NSArray initWithCoder:]`.
pub fn decode_current_array(env: &mut Environment, unarchiver: id) -> Vec<id> {
    let keys: Vec<Uid> = {
        let host_obj = borrow_host_obj(env, unarchiver);
        let objects = host_obj.plist["$objects"].as_array().unwrap();
        let item = &objects[host_obj.current_key.unwrap().get() as usize];
        let keys = item.as_dictionary().unwrap()["NS.objects"]
            .as_array()
            .unwrap();
        keys.iter()
            .map(|value| value.as_uid().copied().unwrap())
            .collect()
    };

    // TODO: do we need to increase the refcount for each object?
    keys.into_iter()
        .map(|key| unarchive_key(env, unarchiver, key))
        .collect()
}
