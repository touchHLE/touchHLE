//! `NSKeyedUnarchiver` and deserialization of its object graph format.
//!
//! I don't know of any resources about this format, but you can figure out how
//! it works from staring at the deserialized plist.

use crate::objc::{id, msg, objc_classes, ClassExports, HostObject};
use crate::Environment;
use plist::{Dictionary, Uid, Value};
use std::collections::HashMap;
use std::path::Path;

struct NSKeyedUnarchiverHostObject {
    plist: Dictionary,
    _current_key: Option<Uid>,
    _already_unarchived: HashMap<Uid, id>,
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
/// This is basically `unarchiveObjectWithFile:` from the proper API.
pub fn unarchive_object_with_file(env: &mut Environment, path: &Path) -> id {
    let plist = Value::from_file(path).unwrap();
    let plist = plist.into_dictionary().unwrap();
    assert!(plist["$version"].as_unsigned_integer() == Some(100000));
    assert!(plist["$archiver"].as_string() == Some("NSKeyedArchiver"));

    let unarchiver = Box::new(NSKeyedUnarchiverHostObject {
        plist,
        _current_key: None,
        _already_unarchived: HashMap::new(),
    });
    let class = env.objc.get_known_class("NSKeyedUnarchiver", &mut env.mem);
    let unarchiver = env.objc.alloc_object(class, unarchiver, &mut env.mem);

    let top_level_object = unarchive_key(env, unarchiver, None);

    let _: () = msg![env; unarchiver release];

    top_level_object
}

/// The core of the implementation: recursively unarchive things.
fn unarchive_key(env: &mut Environment, unarchiver: id, key: Option<Uid>) -> id {
    let host_obj = env.objc.borrow::<NSKeyedUnarchiverHostObject>(unarchiver);
    // TODO: check for key in the already_unarchived map
    let item = if let Some(uid) = key {
        &host_obj.plist["$objects"].as_array().unwrap()[uid.get() as usize]
    } else {
        &host_obj.plist["$top"]
    };
    unimplemented!("Unarchive: {:#?}", item);
    // TODO: process item and construct appropriate object, insert into map
}
