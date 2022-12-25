//! `UINib` and loading of nib files.
//!
//! Resources:
//! - Apple's [Resource Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/LoadingResources/CocoaNibs/CocoaNibs.html) is very helpful.
//! - GitHub user 0xced's [reverse-engineering of UIClassSwapper](https://gist.github.com/0xced/45daf79b62ad6a20be1c).

use crate::frameworks::foundation::ns_keyed_unarchiver;
use crate::frameworks::foundation::ns_string::string_with_rust_string;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports};
use crate::Environment;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// TODO actual UINib class. It's not needed for the main nib file which is
// loaded implicitly.

// An undocumented type that nib files reference by name. NSKeyedUnarchiver will
// find and instantiate this class.
@implementation UIProxyObject: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIProxyObject*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

// Another undocumented type used by nib files. This one seems to be used to
// instantiate types that don't implement NSCoding (i.e. don't respond to
// initWithCoder:). See the link at the top of this file.
@implementation UIClassSwapper: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIClassSwapper*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

// Another undocumented type used by nib files.
@implementation UIRuntimeOutletConnection: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIRuntimeOutletConnection*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

};

/// Shortcut for use by [super::ui_application::UIApplicationMain].
///
/// In terms of the proper API, it should behave something like:
/// ```objc
/// UINib *nib = [UINib nibWithName:main_nib_file bundle:nil];
/// return [nib instantiateWithOwner:[UIApplication sharedApplication]
///                     optionsOrNil:nil];
/// ```
///
/// The result value of this function is the `NSArray` of top-level objects.
pub fn load_main_nib_file(env: &mut Environment, _ui_application: id) -> id {
    let path = env.bundle.main_nib_file_path();

    let unarchiver = msg_class![env; NSKeyedUnarchiver alloc];
    ns_keyed_unarchiver::init_for_reading_file(env, unarchiver, &path);

    // The top-level keys in a nib file's keyed archive appear to be
    // UINibAccessibilityConfigurationsKey, UINibConnectionsKey,
    // UINibObjectsKey, UINibTopLevelObjectsKey and UINibVisibleWindowsKey.
    // Each corresponds to an NSArray.
    //
    // Only the objects, top-level objects and connections lists seem useful
    // right now.

    fn get_key(env: &mut Environment, unarchiver: id, key: &str) -> id {
        let key = string_with_rust_string(env, key.to_string());
        let list: id = msg![env; unarchiver decodeObjectForKey:key];
        let _: () = msg![env; key release];
        list
    }

    let objects = get_key(env, unarchiver, "UINibObjectsKey");
    let connections = get_key(env, unarchiver, "UINibConnectionsKey");
    let top_level_objects = get_key(env, unarchiver, "UINibTopLevelObjectsKey");

    let _: () = msg![env; unarchiver release];

    unimplemented!(
        "Finish nib loading with {:?}, {:?}, {:?}",
        objects,
        connections,
        top_level_objects
    );
}
