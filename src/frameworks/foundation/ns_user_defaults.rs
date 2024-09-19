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
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
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
    // NSMutableDictionary *
    dictionary: id,
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
        dictionary: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: plist methods etc

- (id)init {
    // TODO: Are there other default keys we need to set?
    let langs_value: id = msg_class![env; NSLocale preferredLanguages];
    let langs_key: id = ns_string::get_static_str(env, "AppleLanguages");

    let dict = msg_class![env; NSMutableDictionary dictionary];
    () = msg![env; dict setObject:langs_value forKey:langs_key];

    retain(env, dict);
    env.objc.borrow_mut::<NSUserDefaultsHostObject>(this).dictionary = dict;
    this
}

- (())dealloc {
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).dictionary;
    release(env, dict);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)dictionaryRepresentation { // NSDictionary *
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).dictionary;
    let dict = msg![env; dict copy];
    autorelease(env, dict)
}

- (id)objectForKey:(id)key { // NSString*
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).dictionary;
    msg![env; dict objectForKey:key]
}
- (())setObject:(id)object
         forKey:(id)key { // NSString*
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).dictionary;
    msg![env; dict setObject:object forKey:key]
}
- (())removeObjectForKey:(id)key {
    let dict = env.objc.borrow::<NSUserDefaultsHostObject>(this).dictionary;
    msg![env; dict removeObjectForKey:key]
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

- (id)stringForKey:(id)key {
    let val: id = msg![env; this objectForKey:key];
    if val == nil {
        return nil;
    }
    let ns_string_class = env.objc.get_known_class("NSString", &mut env.mem);
    if env.objc.class_is_subclass_of(val, ns_string_class) {
        return val;
    }
    let ns_number_class = env.objc.get_known_class("NSNumber", &mut env.mem);
    if env.objc.class_is_subclass_of(val, ns_number_class) {
        todo!();
    }
    nil
}

@end

};
