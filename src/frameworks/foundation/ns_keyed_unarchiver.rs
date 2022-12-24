//! `NSKeyedUnarchiver` and deserialization of its object graph format.
//!
//! I don't know of any resources about this format, but you can figure out how
//! it works from staring at the deserialized plist.

use crate::objc::{id, objc_classes, ClassExports};
use crate::Environment;
use plist::Value;
use std::path::Path;

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
pub fn unarchive_object_with_file(_env: &mut Environment, path: &Path) -> id {
    let plist = Value::from_file(path).unwrap();
    let plist = plist.into_dictionary().unwrap();
    assert!(plist["$version"].as_unsigned_integer() == Some(100000));
    assert!(plist["$archiver"].as_string() == Some("NSKeyedArchiver"));

    unimplemented!("Unarchive: {:#?}", plist);
}
