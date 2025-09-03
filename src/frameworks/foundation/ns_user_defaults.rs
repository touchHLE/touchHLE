/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSUserDefaults`.
//!
//! References:
//! - Apple's [Preferences and Settings Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/UserDefaults/AboutPreferenceDomains/AboutPreferenceDomains.html).

use super::{ns_string, NSInteger};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, Class, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

#[derive(Default)]
pub struct State {
    /// `NSUserDefaults*`
    standard_defaults: Option<id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_user_defaults
    }
}

struct NSUserDefaultsHostObject {
    /// Defaults meant to be seen by all applications.
    /// *Does NOT* persist on disk.
    /// `NSMutableDictionary *`
    global_domain_dict: id,
    /// Application own preferences.
    /// *Does* persist on disk.
    /// `NSMutableDictionary *`
    app_domain_dict: id,
    /// Application temporary defaults.
    /// *Does NOT* persist on disk.
    /// Used if not found in other dictionaries.
    /// `NSMutableDictionary *`
    registration_domain_dict: id,
}
impl HostObject for NSUserDefaultsHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUserDefaults: NSObject

+ (id)standardUserDefaults {
    if let Some(existing) = State::get(env).standard_defaults {
        existing
    } else {
        let defaults = msg![env; this alloc];
        let defaults = msg![env; defaults init];
        State::get(env).standard_defaults = Some(defaults);
        defaults
    }
}

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSUserDefaultsHostObject {
        global_domain_dict: nil,
        app_domain_dict: nil,
        registration_domain_dict: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: plist methods etc

- (id)init {
    // First, init globals
    // TODO: init globals once per app run
    // TODO: Are there other default keys we need to set?
    let langs_value: id = msg_class![env; NSLocale preferredLanguages];
    let langs_key: id = ns_string::get_static_str(env, "AppleLanguages");

    let dict = msg_class![env; NSMutableDictionary new];
    () = msg![env; dict setObject:langs_value forKey:langs_key];

    env.objc.borrow_mut::<NSUserDefaultsHostObject>(this).global_domain_dict = dict;

    // Now, load from disk and init app's own preferences.
    let plist_file_name = format!("{}.plist", env.bundle.bundle_identifier());
    let plist_file_path_buf = env.fs.home_directory()
        .join("Library")
        .join("Preferences")
        .join(plist_file_name);
    let plist_file_path = ns_string::from_rust_string(env, plist_file_path_buf.as_str().to_string());
    let dict: id = msg_class![env; NSDictionary dictionaryWithContentsOfFile:plist_file_path];

    let dict: id = if dict == nil {
        msg_class![env; NSMutableDictionary new]
    } else {
        msg![env; dict mutableCopy]
    };
    env.objc.borrow_mut::<NSUserDefaultsHostObject>(this).app_domain_dict = dict;

    this
}

- (())dealloc {
    let app_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    release(env, app_domain_dict);
    let global_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).global_domain_dict;
    release(env, global_domain_dict);
    let registration_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).registration_domain_dict;
    release(env, registration_domain_dict);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)dictionaryRepresentation { // NSDictionary *
    let dict = msg_class![env; NSMutableDictionary new];
    let registration_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).registration_domain_dict;
    if registration_domain_dict != nil {
        () = msg![env; dict addEntriesFromDictionary:registration_domain_dict];
    }
    let global_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).global_domain_dict;
    () = msg![env; dict addEntriesFromDictionary:global_domain_dict];
    let app_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    () = msg![env; dict addEntriesFromDictionary:app_domain_dict];
    autorelease(env, dict)
}

- (id)valueForKey:(id)key { // NSString*
    // This is not documented, but apparently this method
    // is overridden for NSUserDefaults.
    // Behaviour was confirmed on macOS.
    // TODO: should we call `valueForKey:` on the app_domain_dict instead?
    msg![env; this objectForKey:key]
}
- (())setValue:(id)val
        forKey:(id)key { // NSString*
    // This is not documented, but apparently this method
    // is overridden for NSUserDefaults.
    // Behaviour was confirmed on macOS.
    // Only app domain gets affected here (this part wasn't verified).
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    msg![env; dict setValue:val forKey:key]
}

- (id)objectForKey:(id)key { // NSString*
    // TODO: check if order of searching is correct
    let app_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    let res: id = msg![env; app_domain_dict objectForKey:key];
    if res != nil {
        return res;
    }
    let global_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).global_domain_dict;
    let res = msg![env; global_domain_dict objectForKey:key];
    if res != nil {
        return res;
    }
    let registration_domain_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).registration_domain_dict;
    msg![env; registration_domain_dict objectForKey:key]
}

- (())registerDefaults:(id)registration_dictionary {
    let reg_dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).registration_domain_dict;
    let dict = if reg_dict != nil {
        reg_dict
    } else {
        let new_dict = msg_class![env; NSMutableDictionary new];
        env.objc.borrow_mut::<NSUserDefaultsHostObject>(this).registration_domain_dict = new_dict;
        new_dict
    };

    // Add new defaults and replace any already defined ones
    () = msg![env; dict addEntriesFromDictionary:registration_dictionary];
}

- (())setObject:(id)object
         forKey:(id)key { // NSString*
    // Only app domain gets affected!
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    msg![env; dict setObject:object forKey:key]
}

- (())removeObjectForKey:(id)key {
    // Only app domain gets affected!
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    msg![env; dict removeObjectForKey:key]
}

- (id)dataForKey:(id)key {
    let val: id = msg![env; this objectForKey:key];
    if val == nil {
        return nil;
    }
    let class: Class = msg![env; val class];
    assert!(class != nil);
    let ns_data_class = env.objc.get_known_class("NSData", &mut env.mem);
    assert!(env.objc.class_is_subclass_of(class, ns_data_class));
    val
}

- (bool)boolForKey:(id)key { // NSString *
    let val: id = msg![env; this objectForKey:key];
    msg![env; val boolValue]
}
- (())setBool:(bool)value
       forKey:(id)key { // NSString *
    let num: id = msg_class![env; NSNumber numberWithBool:value];
    msg![env; this setObject:num forKey:key]
}

- (NSInteger)integerForKey:(id)key {
    let val: id = msg![env; this objectForKey:key];
    msg![env; val integerValue]
}
- (())setInteger:(NSInteger)value
          forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithInteger:value];
    msg![env; this setObject:num forKey:key]
}

- (f32)floatForKey:(id)key {
    let val: id = msg![env; this objectForKey:key];
    msg![env; val floatValue]
}
- (())setFloat:(f32)value
        forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithFloat:value];
    msg![env; this setObject:num forKey:key]
}

- (f64)doubleForKey:(id)key {
    let val: id = msg![env; this objectForKey:key];
    msg![env; val doubleValue]
}
- (())setDouble:(f64)value
          forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithDouble:value];
    msg![env; this setObject:num forKey:key]
}

- (id)stringForKey:(id)key {
    log_dbg!("NSUserDefaults stringForKey:{}", to_rust_string(env, key));
    let val: id = msg![env; this objectForKey:key];
    if val == nil {
        return nil;
    }
    let val_class: Class = msg![env; val class];
    let ns_string_class = env.objc.get_known_class("NSString", &mut env.mem);
    if env.objc.class_is_subclass_of(val_class, ns_string_class) {
        return val;
    }
    let ns_number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    if env.objc.class_is_subclass_of(val_class, ns_number_class) {
        todo!();
    }
    nil
}

- (id)arrayForKey:(id)key {
    log_dbg!("NSUserDefaults arrayForKey:{}", to_rust_string(env, key));
    let val: id = msg![env; this objectForKey:key];
    if val == nil {
        return nil;
    }
    let val_class: Class = msg![env; val class];
    let ns_array_class = env.objc.get_known_class("NSArray", &mut env.mem);
    if env.objc.class_is_subclass_of(val_class, ns_array_class) {
        return val;
    }
    nil
}

- (bool)synchronize {
    // Note: only app domain dict gets synchronized!
    let plist_file_path_dir = env.fs.home_directory()
        .join("Library")
        .join("Preferences");
    // TODO: can we avoid this creation call on each sync?
    _ = env.fs.create_dir_all(plist_file_path_dir.clone());
    let plist_file_name = format!("{}.plist", env.bundle.bundle_identifier());
    let plist_file_path_buf = plist_file_path_dir.join(plist_file_name);
    let plist_file_path = ns_string::from_rust_string(env, plist_file_path_buf.as_str().to_string());
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).app_domain_dict;
    msg![env; dict writeToFile:plist_file_path atomically:true]
}

@end

};
