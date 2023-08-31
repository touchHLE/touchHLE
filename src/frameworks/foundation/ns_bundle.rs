/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSBundle`.

use super::ns_string;
use crate::bundle::Bundle;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
};

#[derive(Default)]
pub struct State {
    main_bundle: Option<id>,
}

struct NSBundleHostObject {
    /// If this is [None], this is the main bundle's NSBundle instance and the
    /// [Bundle] is stored in [crate::Environment], not here.
    _bundle: Option<Bundle>,
    /// NSString with bundle path.
    bundle_path: id,
    /// NSURL with bundle path. [None] if not created yet.
    bundle_url: Option<id>,
    /// `NSDictionary*` for the `Info.plist` content. [None] if not created yet.
    info_dictionary: Option<id>,
}
impl HostObject for NSBundleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSBundle: NSObject

+ (id)mainBundle {
    if let Some(bundle) = env.framework_state.foundation.ns_bundle.main_bundle {
        bundle
    } else {
        let bundle_path = env.bundle.bundle_path().as_str().to_string();
        let bundle_path = ns_string::from_rust_string(env, bundle_path);
        let host_object = NSBundleHostObject {
            _bundle: None,
            bundle_path,
            bundle_url: None,
            info_dictionary: None,
        };
        let new = env.objc.alloc_object(
            this,
            Box::new(host_object),
            &mut env.mem
        );
        env.framework_state.foundation.ns_bundle.main_bundle = Some(new);
        new
   }
}

- (())dealloc {
    let &NSBundleHostObject {
        _bundle: _,
        bundle_path: _, // FIXME?
        bundle_url,
        info_dictionary,
    } = env.objc.borrow(this);
    if let Some(bundle_url) = bundle_url {
        release(env, bundle_url);
    }
    if let Some(info_dictionary) = info_dictionary {
        release(env, info_dictionary);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)bundlePath {
    env.objc.borrow::<NSBundleHostObject>(this).bundle_path
}
- (id)bundleURL {
    if let Some(url) = env.objc.borrow::<NSBundleHostObject>(this).bundle_url {
        url
    } else {
        let bundle_path: id = msg![env; this bundlePath];
        let new: id = msg_class![env; NSURL alloc];
        let new: id = msg![env; new initFileURLWithPath:bundle_path];
        env.objc.borrow_mut::<NSBundleHostObject>(this).bundle_url = Some(new);
        new
    }
}

- (id)resourcePath {
    // This seems to be the same as the bundle path. The iPhone OS bundle
    // structure is a lot flatter than the macOS one.
    msg![env; this bundlePath]
}
- (id)resourceURL {
    // This seems to be the same as the bundle path. The iPhone OS bundle
    // structure is a lot flatter than the macOS one.
    msg![env; this bundleURL]
}

- (id)pathForResource:(id)name // NSString*
               ofType:(id)extension // NSString*
          inDirectory:(id)directory { // NSString*
    assert!(name != nil); // TODO

    // FIXME: localized resource handling?
    // FIXME: return nil if path does not exist

    let mut path: id = msg![env; this resourcePath];
    if directory != nil {
        path = msg![env; path stringByAppendingPathComponent:directory];
    }
    path = msg![env; path stringByAppendingPathComponent:name];
    if extension != nil {
        path = msg![env; path stringByAppendingPathExtension:extension];
    }
    path
}
- (id)pathForResource:(id)name // NSString*
               ofType:(id)extension { // NSString*
    // TODO: move all of this to pathForResource:ofType:inDirectory: ?
    // not clear from docs if it is the case
    let path = msg![env; this pathForResource:name ofType:extension inDirectory:nil];
    let file_manager: id = msg_class![env; NSFileManager defaultManager];
    let file_exists: bool = msg![env; file_manager fileExistsAtPath:path];
    if file_exists {
        return path;
    }
    let mut l_path: id = msg![env; this resourcePath];
    // TODO: other localizations
    let lproj: id = ns_string::get_static_str(env, "English.lproj");
    l_path = msg![env; l_path stringByAppendingPathComponent:lproj];
    l_path = msg![env; l_path stringByAppendingPathComponent:name];
    if extension != nil {
        l_path = msg![env; l_path stringByAppendingPathExtension:extension];
    }
    let l_file_exists: bool = msg![env; file_manager fileExistsAtPath:l_path];
    if l_file_exists {
        return l_path;
    }
    nil
}
- (id)URLForResource:(id)name // NSString*
       withExtension:(id)extension // NSString *
        subdirectory:(id)subpath { // NSString *
   let path_string: id = msg![env; this pathForResource:name
                                                 ofType:extension
                                            inDirectory:subpath];
   let path_url: id = msg_class![env; NSURL alloc];
   let path_url: id = msg![env; path_url initFileURLWithPath:path_string];
   autorelease(env, path_url)
}
- (id)URLForResource:(id)name // NSString*
       withExtension:(id)extension { // NSString *
   msg![env; this URLForResource:name withExtension:extension subdirectory:nil]
}

- (id)infoDictionary {
    let &NSBundleHostObject {
        bundle_path,
        info_dictionary,
        ..
    } = env.objc.borrow(this);
    if let Some(dict) = info_dictionary {
        return dict;
    }

    let plist_path = ns_string::get_static_str(env, "Info.plist");
    let plist_path: id = msg![env; bundle_path stringByAppendingPathComponent:plist_path];
    let dict: id = msg_class![env; NSDictionary alloc];
    let dict: id = msg![env; dict initWithContentsOfFile:plist_path];
    env.objc.borrow_mut::<NSBundleHostObject>(this).info_dictionary = Some(dict);
    dict
}

// TODO: constructors, more accessors

@end

};
