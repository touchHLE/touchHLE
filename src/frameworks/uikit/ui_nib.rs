//! `UINib` and loading of nib files.
//!
//! Resources:
//! - Apple's [Resource Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/LoadingResources/CocoaNibs/CocoaNibs.html) is very helpful.
//! - GitHub user 0xced's [reverse-engineering of UIClassSwapper](https://gist.github.com/0xced/45daf79b62ad6a20be1c).

use crate::frameworks::foundation::ns_keyed_unarchiver;
use crate::frameworks::foundation::ns_string::{copy_string, string_with_static_str};
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
    log!("TODO: [(UIProxyObject*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

// Another undocumented type used by nib files. This one seems to be used to
// instantiate types that don't implement NSCoding (i.e. don't respond to
// initWithCoder:). See the link at the top of this file.
@implementation UIClassSwapper: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {

    let name_key = string_with_static_str(env, "UIClassName");
    let name_nss: id = msg![env; coder decodeObjectForKey:name_key];
    let name = copy_string(env, name_nss);
    let _: () = msg![env; name_nss release];

    let orig_key = string_with_static_str(env, "UIOriginalClassName");
    let orig_nss: id = msg![env; coder decodeObjectForKey:orig_key];
    let orig = copy_string(env, orig_nss);
    let _: () = msg![env; orig_nss release];

    let class = env.objc.get_known_class(&name, &mut env.mem);

    let object: id = msg![env; class alloc];
    let object: id = if orig == "UICustomObject" {
        msg![env; object init]
    } else {
        msg![env; object initWithCoder:coder]
    };
    let _: () = msg![env; this release];
    // TODO: autorelease the object?
    object
}

@end

// Another undocumented type used by nib files.
@implementation UIRuntimeOutletConnection: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("TODO: [(UIRuntimeOutletConnection*){:?} initWithCoder:{:?}]", this, coder);
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

    let objects_key = string_with_static_str(env, "UINibObjectsKey");
    let objects: id = msg![env; unarchiver decodeObjectForKey:objects_key];
    let conns_key = string_with_static_str(env, "UINibConnectionsKey");
    let conns: id = msg![env; unarchiver decodeObjectForKey:conns_key];
    let tlos_key = string_with_static_str(env, "UINibTopLevelObjectsKey");
    let tlos: id = msg![env; unarchiver decodeObjectForKey:tlos_key];

    let _: () = msg![env; unarchiver release];

    unimplemented!(
        "Finish nib loading with {:?}, {:?}, {:?}",
        objects,
        conns,
        tlos,
    );
}
