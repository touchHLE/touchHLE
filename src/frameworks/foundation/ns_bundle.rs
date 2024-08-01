/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSBundle`.

use super::{ns_string, NSUInteger};
use crate::bundle::Bundle;
use crate::frameworks::core_foundation::cf_bundle::{
    CFBundleCopyBundleLocalizations, CFBundleCopyPreferredLocalizationsFromArray,
};
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::frameworks::uikit::ui_nib::load_nib_file;
use crate::fs::GuestPathBuf;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
};
use crate::Environment;
use std::collections::{HashMap, HashSet};

// Should be ISO 639-1 (or ISO 639-2) compliant
// TODO: complete this list or use some crate for mapping
const LANG_ID_TO_LANG_PROJ: &[(&str, &str)] = &[
    ("da", "Danish.lproj"),
    ("nl", "Dutch.lproj"),
    ("en", "English.lproj"),
    ("fi", "Finnish.lproj"),
    ("fr", "French.lproj"),
    ("de", "German.lproj"),
    ("it", "Italian.lproj"),
    ("ja", "Japanese.lproj"),
    ("no", "Norwegian.lproj"),
    ("es", "Spanish.lproj"),
    ("sv", "Swedish.lproj"),
];

#[derive(Default)]
pub struct State {
    main_bundle: Option<id>,
    localization_tables: HashMap<id, id>, // NSString* to NSDictionary*
}

pub struct NSBundleHostObject {
    /// If this is [None], this is the main bundle's NSBundle instance and the
    /// [Bundle] is stored in [crate::Environment], not here.
    pub bundle: Option<Bundle>,
    /// NSString with bundle path.
    bundle_path: id,
    /// NSString with bundle identifier.
    bundle_identifier: id,
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
        let bundle_identifier = env.bundle.bundle_identifier().to_string();
        let bundle_identifier = ns_string::from_rust_string(env, bundle_identifier);
        let host_object = NSBundleHostObject {
            bundle: None,
            bundle_path,
            bundle_identifier,
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

+ (id)preferredLocalizationsFromArray:(id)localizations_array { // NSArray<NSString *> *
    let preferredLocalizations = CFBundleCopyPreferredLocalizationsFromArray(env, localizations_array);
    autorelease(env, preferredLocalizations)
}

- (())dealloc {
    let &NSBundleHostObject {
        bundle: _,
        bundle_path: _, // FIXME?
        bundle_identifier: _, // FIXME?
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
- (id)bundleIdentifier {
    env.objc.borrow::<NSBundleHostObject>(this).bundle_identifier
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

- (id)loadNibNamed:(id)name // NSString*
             owner:(id)_owner
           options:(id)options { // NSDictionary<UINibOptionsKey, id> *
    if !options.is_null() {
        let options_count: NSUInteger = msg![env; options count];
        assert!(options_count == 0);
    }
    let name_string = to_rust_string(env, name);
    let bundle_path = to_rust_string(env, env.objc.borrow::<NSBundleHostObject>(this).bundle_path);
    let nib_path = format!("{}/{}.nib", bundle_path, name_string);
    let unarchiver = load_nib_file(env, GuestPathBuf::from(nib_path)).unwrap(); // TODO: Set owner and use options
    let top_level_objects_key = get_static_str(env, "UINibTopLevelObjectsKey");
    let top_level_objects = msg![env; unarchiver decodeObjectForKey:top_level_objects_key];
    release(env, unarchiver);
    top_level_objects
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

- (id)executablePath {
    let exec_path_str = env.bundle.executable_path().as_str().to_string();
    let exec_path = from_rust_string(env, exec_path_str);
    autorelease(env, exec_path)
}

- (id)pathForResource:(id)name // NSString*
               ofType:(id)extension // NSString*
          inDirectory:(id)directory { // NSString*
    assert!(name != nil); // TODO

    // TODO: cache result of lookups

    let path = path_for_resource_helper(env, this, name, nil, directory, extension);
    if path != nil {
        return path
    }

    // Try preferred languages in order of preference
    let langs: id = msg_class![env; NSLocale preferredLanguages];
    let lang_count: NSUInteger = msg![env; langs count];
    let mut unknown_codes = HashSet::new();
    for i in 0..lang_count {
        let lang_code: id = msg![env; langs objectAtIndex:i];
        let lang_code = ns_string::to_rust_string(env, lang_code); // TODO: avoid copy
        if let Some(&(_, lproj)) = LANG_ID_TO_LANG_PROJ.iter().find(|&&(code, _)| code == lang_code) {
            let lproj: id = ns_string::get_static_str(env, lproj);
            let localized_path = path_for_resource_helper(env, this, name, lproj, directory, extension);
            if localized_path != nil {
                return localized_path;
            }
        } else {
            unknown_codes.insert(lang_code);
        }
    }

    // As a last resort, fallback to English
    // TODO: fallback to a development language (CFBundleDevelopmentRegion from
    // Info.plist)
    if !unknown_codes.is_empty() {
        log!("TODO: language codes {:?} aren't mapped to a language name, falling back to English", unknown_codes);
    }
    let lproj: id = ns_string::get_static_str(env, "English.lproj");
    path_for_resource_helper(env, this, name, lproj, directory, extension)
}
- (id)pathForResource:(id)name // NSString*
               ofType:(id)extension { // NSString*
    msg![env; this pathForResource:name ofType:extension inDirectory:nil]
}
- (id)URLForResource:(id)name // NSString*
       withExtension:(id)extension // NSString *
        subdirectory:(id)subpath { // NSString *
    let path_string: id = msg![env; this pathForResource:name
                                                 ofType:extension
                                            inDirectory:subpath];
    if path_string == nil {
        return nil;
    }
    let path_url: id = msg_class![env; NSURL alloc];
    let path_url: id = msg![env; path_url initFileURLWithPath:path_string];
    autorelease(env, path_url)
}
- (id)URLForResource:(id)name // NSString*
       withExtension:(id)extension { // NSString *
    msg![env; this URLForResource:name withExtension:extension subdirectory:nil]
}

- (id)localizedStringForKey:(id)key
                      value:(id)value
                      table:(id)tableName {
    log_dbg!("localizedStringForKey key:'{}' value:'{}' table:'{}'",
            if key == nil { std::borrow::Cow::from("(null)") } else { ns_string::to_rust_string(env, key) },
            if value == nil { std::borrow::Cow::from("(null)") } else { ns_string::to_rust_string(env, value) },
            if tableName == nil { std::borrow::Cow::from("(null)") } else { ns_string::to_rust_string(env, tableName) }
    );
    let empty_str: id = ns_string::get_static_str(env, "");
    if key == nil {
        if value == nil {
            return empty_str;
        }
        return value;
    }
    let name = if tableName == nil {
        ns_string::get_static_str(env, "Localizable")
    } else {
        tableName
    };
    // TODO: support arbitrary bundles, not only main one
    assert_eq!(this, env.framework_state.foundation.ns_bundle.main_bundle.unwrap());
    let dict = if let Some(&table_dict) = env.framework_state.foundation.ns_bundle.localization_tables.get(&name) {
        table_dict
    } else {
        let extension = ns_string::get_static_str(env, "strings");
        let dict_url: id = msg![env; this URLForResource:name withExtension:extension];
        let dict: id = msg_class![env; NSDictionary dictionaryWithContentsOfURL:dict_url];
        assert!(dict != nil);
        retain(env, name);
        retain(env, dict);
        env.framework_state.foundation.ns_bundle.localization_tables.insert(name, dict);
        dict
    };
    let res: id = msg![env; dict objectForKey:key];
    if res == nil {
        if value == nil || value == empty_str {
            return key;
        }
        return value;
    }
    log_dbg!("localizedStringForKey res => {:?}", ns_string::to_rust_string(env, res));
    res
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

- (id)objectForInfoDictionaryKey:(id)key {
    let info_dict = msg![env; this infoDictionary];
    // TODO: return the localized value of a key when one is available
    msg![env; info_dict objectForKey:key]
}

- (id)localizations {
    let localizations = CFBundleCopyBundleLocalizations(env, this);
    autorelease(env, localizations)
}

// TODO: constructors, more accessors

@end

};

fn path_for_resource_helper(
    env: &mut Environment,
    bundle: id,
    name: id,
    lproj: id,
    directory: id,
    extension: id,
) -> id {
    let mut path: id = msg![env; bundle resourcePath];
    if lproj != nil {
        path = msg![env; path stringByAppendingPathComponent:lproj];
    }
    if directory != nil {
        path = msg![env; path stringByAppendingPathComponent:directory];
    }
    path = msg![env; path stringByAppendingPathComponent:name];
    if extension != nil {
        path = msg![env; path stringByAppendingPathExtension:extension];
    }
    let file_manager: id = msg_class![env; NSFileManager defaultManager];
    let file_exists: bool = msg![env; file_manager fileExistsAtPath:path];
    if file_exists {
        return path;
    }
    nil
}
