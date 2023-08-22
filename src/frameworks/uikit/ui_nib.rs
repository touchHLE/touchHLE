/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINib` and loading of nib files.
//!
//! Resources:
//! - Apple's [Resource Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/LoadingResources/CocoaNibs/CocoaNibs.html) is very helpful.
//! - GitHub user 0xced's [reverse-engineering of UIClassSwapper](https://gist.github.com/0xced/45daf79b62ad6a20be1c).

use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::frameworks::foundation::{ns_keyed_unarchiver, NSUInteger};
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
};
use crate::Environment;

struct UIRuntimeOutletConnectionHostObject {
    destination: id,
    label: id,
    source: id,
}
impl HostObject for UIRuntimeOutletConnectionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// TODO actual UINib class. It's not needed for the main nib file which is
// loaded implicitly.

// An undocumented type that nib files reference by name. NSKeyedUnarchiver will
// find and instantiate this class.
@implementation UIProxyObject: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let id_key = get_static_str(env, "UIProxiedObjectIdentifier");
    let id_nss: id = msg![env; coder decodeObjectForKey:id_key];
    let id = to_rust_string(env, id_nss);

    if id == "IBFilesOwner" {
        // The file owner is usually the UIApplication instance.
        // Replacing the proxy with that instance is important so that the
        // "delegate" outlet can be connected between it and the
        // UIApplicationDelegate.
        //
        // TODO: This is a bit of a hack. Eventually it would be good to fix:
        // - The name "UIProxyObject" implies that it might be intended to
        //   proxy messages to another object, rather than be replaced by it.
        //   Check what iPhone OS does?
        // - If/when the UINib class is implemented and arbitrary nib files can
        //   be deserialized, an app could pick some other object to be the nib
        //   file owner, which this would need to handle.
        // - If this object is meant to be replaced, it's probably not meant to
        //   be done via `initWithCoder:`, but instead by providing a delegate
        //   to the NSKeyedUnarchiver. That might be needed to implement
        //   replacement for objects other than the UIApplication instance.

        release(env, this);
        msg_class![env; UIApplication sharedApplication]
    } else {
        log!("TODO: UIProxyObject replacement for {}, instance {:?} left unreplaced", id, this);
        this
    }
}

@end

// Another undocumented type used by nib files. This one seems to be used to
// instantiate types that don't implement NSCoding (i.e. don't respond to
// initWithCoder:). See the link at the top of this file.
@implementation UIClassSwapper: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {

    let name_key = get_static_str(env, "UIClassName");
    let name_nss: id = msg![env; coder decodeObjectForKey:name_key];
    let name = to_rust_string(env, name_nss);

    let orig_key = get_static_str(env, "UIOriginalClassName");
    let orig_nss: id = msg![env; coder decodeObjectForKey:orig_key];
    let orig = to_rust_string(env, orig_nss);

    let class = env.objc.get_known_class(&name, &mut env.mem);

    let object: id = msg![env; class alloc];
    let object: id = if orig == "UICustomObject" {
        msg![env; object init]
    } else {
        msg![env; object initWithCoder:coder]
    };
    release(env, this);
    // TODO: autorelease the object?
    object
}

@end

// Another undocumented type used by nib files. This one's purpose seems to be
// to connect outlets once all the objects are deserialized.
@implementation UIRuntimeOutletConnection: NSObject

+ (id)alloc {
    let host_object = Box::new(UIRuntimeOutletConnectionHostObject {
        destination: nil,
        label: nil,
        source: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {

    let destination_key = get_static_str(env, "UIDestination");
    let destination: id = msg![env; coder decodeObjectForKey: destination_key];

    let label_key = get_static_str(env, "UILabel");
    let label: id = msg![env; coder decodeObjectForKey: label_key];

    let source_key = get_static_str(env, "UISource");
    let source: id = msg![env; coder decodeObjectForKey: source_key];

    retain(env, destination);
    retain(env, source);
    retain(env, label);
    let host_obj = env.objc.borrow_mut::<UIRuntimeOutletConnectionHostObject>(this);
    host_obj.destination = destination;
    host_obj.label = label;
    host_obj.source = source;

    this
}

- (())connect {
    let &UIRuntimeOutletConnectionHostObject {
        destination,
        label,
        source
    } = env.objc.borrow(this);

    () = msg![env; source setValue:destination forKey:label];
}

- (())dealloc {
    let &UIRuntimeOutletConnectionHostObject {
        destination,
        label,
        source
    } = env.objc.borrow(this);
    release(env, destination);
    release(env, label);
    release(env, source);

    env.objc.dealloc_object(this, &mut env.mem)
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
pub fn load_main_nib_file(env: &mut Environment, _ui_application: id) {
    let Some(path) = env.bundle.main_nib_file_path() else {
        return;
    };

    let Ok(data) = env.fs.read(path) else {
        // Apparently it's permitted to specify the nib file key in the
        // Info.plist, yet not have it point to a valid nib file?!
        log!("Warning: couldn't load main nib file");
        return;
    };

    let unarchiver = msg_class![env; NSKeyedUnarchiver alloc];
    ns_keyed_unarchiver::init_for_reading_with_data(env, unarchiver, &data);

    // The top-level keys in a nib file's keyed archive appear to be
    // UINibAccessibilityConfigurationsKey, UINibConnectionsKey,
    // UINibObjectsKey, UINibTopLevelObjectsKey and UINibVisibleWindowsKey.
    // Each corresponds to an NSArray.

    // We don't need to do anything with the list of objects, but deserializing
    // it ensures everything else is deserialized.
    let objects_key = get_static_str(env, "UINibObjectsKey");
    let _objects: id = msg![env; unarchiver decodeObjectForKey:objects_key];

    // Connect all the outlets with UIRuntimeOutletConnection
    let conns_key = get_static_str(env, "UINibConnectionsKey");
    let conns: id = msg![env; unarchiver decodeObjectForKey:conns_key];
    let conns_count: NSUInteger = msg![env; conns count];
    for i in 0..conns_count {
        let conn: id = msg![env; conns objectAtIndex:i];
        () = msg![env; conn connect];
    }

    // Make visible windows visible
    let visibles_key = get_static_str(env, "UINibVisibleWindowsKey");
    let visibles: id = msg![env; unarchiver decodeObjectForKey:visibles_key];
    let visibles_count: NSUInteger = msg![env; visibles count];
    for i in 0..visibles_count {
        let visible: id = msg![env; visibles objectAtIndex:i];
        () = msg![env; visible setHidden:false];
    }

    release(env, unarchiver);
}
