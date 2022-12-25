//! `NSKeyedUnarchiver` and deserialization of its object graph format.
//!
//! Resources:
//! - You can get a good intuitive grasp of how the format works just by staring
//!   at a pretty-print of a simple nib file from something that can parse
//!   plists, e.g. `plutil -p` or `println!("{:#?}", plist::Value::...);`.
//! - Apple's [Archives and Serializations Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Archiving/Articles/archives.html)

use super::ns_string::copy_string;
use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, objc_classes, ClassExports, HostObject};
use crate::Environment;
use plist::{Dictionary, Uid, Value};
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

+ (id)allocWithZone:(MutVoidPtr)_zone { // struct _NSZone*
    let unarchiver = Box::new(NSKeyedUnarchiverHostObject {
        plist: Dictionary::new(),
        current_key: None,
        already_unarchived: Vec::new(),
    });
    env.objc.alloc_object(this, unarchiver, &mut env.mem)
}

// TODO: real init methods. This is currently only initialized by the shortcut
// function below.

- (())dealloc {
    let host_obj = borrow_host_obj(env, this);
    let already_unarchived = std::mem::take(&mut host_obj.already_unarchived);

    for &object in already_unarchived.iter().flatten() {
        let _: () = msg![env; object release];
    }

    // FIXME: this should do a super-call instead
    env.objc.dealloc_object(this, &mut env.mem)
}

// These methods drive most of the decoding. They get called in two cases:
// - By the code that initiates the unarchival, e.g. UINib, to retrieve
//   top-level objects.
// - By the object currently being unarchived, i.e. something that had
//   `initWithCoder:` called on it, to retrieve objects from its scope.
// They are all from the NSCoder abstract class and they return default values
// if the key is unknown.

- (id)decodeObjectForKey:(id)key { // NSString*
    let key = copy_string(env, key); // TODO: avoid copying string
    let host_obj = borrow_host_obj(env, this);
    let scope = match host_obj.current_key {
        Some(current_uid) => {
            &host_obj.plist["$objects"].as_array().unwrap()[current_uid.get() as usize]
        },
        None => {
            &host_obj.plist["$top"]
        }
    }.as_dictionary().unwrap();
    let next_uid = scope[&key].as_uid().copied().unwrap();
    let object = unarchive_key(env, this, next_uid);
    msg![env; object retain] // caller must release it
}

// TODO: add more decode methods

@end

};

fn borrow_host_obj(env: &mut Environment, unarchiver: id) -> &mut NSKeyedUnarchiverHostObject {
    env.objc.borrow_mut(unarchiver)
}

/// Shortcut for use by [crate::frameworks::uikit::ui_nib::load_main_nib_file].
///
/// This is equivalent to calling `initForReadingWithData:` in the proper API.
pub fn init_for_reading_file(env: &mut Environment, unarchiver: id, path: &Path) {
    // Should have already been alloc'd the proper way.
    let host_obj = borrow_host_obj(env, unarchiver);
    assert!(host_obj.already_unarchived.is_empty());
    assert!(host_obj.current_key.is_none());
    assert!(host_obj.plist.is_empty());

    let plist = Value::from_file(path).unwrap();
    let plist = plist.into_dictionary().unwrap();
    assert!(plist["$version"].as_unsigned_integer() == Some(100000));
    assert!(plist["$archiver"].as_string() == Some("NSKeyedArchiver"));

    let key_count = plist["$objects"].as_array().unwrap().len();

    host_obj.already_unarchived = vec![None; key_count];
    host_obj.plist = plist;
}

/// The core of the implementation: unarchive something by its uid.
///
/// This is recursive in practice: the `initWithCoder:` messages sent by this
/// function will be received by objects which will then send
/// `decodeXXXWithKey:` messages back to the unarchiver, which will then call
/// this function (and so on).
///
/// The object returned will have a refcount of 1 and should be considered
/// owned by the NSKeyedUnarchiver.
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
            new_object
        }
        _ => unimplemented!("Unarchive: {:#?}", item),
    }
}

/// Shortcut for use by `[_touchHLE_NSArray initWithCoder:]`.
///
/// The objects are to be considered retained by the `Vec`.
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

    keys.into_iter()
        .map(|key| {
            let new_object = unarchive_key(env, unarchiver, key);
            // object is retained by the Vec
            let new_object: id = msg![env; new_object retain];
            new_object
        })
        .collect()
}
