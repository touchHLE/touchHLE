/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//! `NSUbiquitousKeyValueStore`

use crate::objc::{id, msg, msg_class, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    pub default_store: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUbiquitousKeyValueStore: NSObject

+ (id)defaultStore {
    // Паттерн синглтона, как в NSFileManager
    if let Some(existing) = env.framework_state.foundation.ns_ubiquitous_key_value_store.default_store {
        existing
    } else {
        let new: id = msg![env; this new];
        env.framework_state.foundation.ns_ubiquitous_key_value_store.default_store = Some(new);
        new
    }
}

- (bool)synchronize {
    // Используем локальное хранилище вместо iCloud
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults synchronize]
}

- (id)objectForKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults objectForKey:key]
}

- (())setObject:(id)obj forKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults setObject:obj forKey:key]
}

- (id)stringForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj == crate::objc::nil {
        return crate::objc::nil;
    }
    // Mirror NSUserDefaults: return the object only if it's actually an
    // NSString (per
    // <https://developer.apple.com/documentation/foundation/nsubiquitouskeyvaluestore/1413946-stringforkey>).
    let ns_string_class = env.objc.get_known_class("NSString", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_string_class] {
        obj
    } else {
        crate::objc::nil
    }
}

- (())setString:(id)value forKey:(id)key {
    () = msg![env; this setObject:value forKey:key];
}

- (id)dataForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj == crate::objc::nil { return crate::objc::nil; }
    let ns_data_class = env.objc.get_known_class("NSData", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_data_class] { obj } else { crate::objc::nil }
}
- (())setData:(id)value forKey:(id)key {
    () = msg![env; this setObject:value forKey:key];
}

- (id)arrayForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj == crate::objc::nil { return crate::objc::nil; }
    let ns_array_class = env.objc.get_known_class("NSArray", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_array_class] { obj } else { crate::objc::nil }
}
- (())setArray:(id)value forKey:(id)key {
    () = msg![env; this setObject:value forKey:key];
}

- (id)dictionaryForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj == crate::objc::nil { return crate::objc::nil; }
    let ns_dict_class = env.objc.get_known_class("NSDictionary", &mut env.mem);
    if msg![env; obj isKindOfClass:ns_dict_class] { obj } else { crate::objc::nil }
}
- (())setDictionary:(id)value forKey:(id)key {
    () = msg![env; this setObject:value forKey:key];
}

- (bool)boolForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj != crate::objc::nil { msg![env; obj boolValue] } else { false }
}
- (())setBool:(bool)value forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithBool:value];
    () = msg![env; this setObject:num forKey:key];
}

- (f64)doubleForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj != crate::objc::nil { msg![env; obj doubleValue] } else { 0.0 }
}
- (())setDouble:(f64)value forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithDouble:value];
    () = msg![env; this setObject:num forKey:key];
}

- (i64)longLongForKey:(id)key {
    let obj: id = msg![env; this objectForKey:key];
    if obj != crate::objc::nil {
        msg![env; obj longLongValue]
    } else {
        0
    }
}

- (())setLongLong:(i64)value forKey:(id)key {
    let num: id = msg_class![env; NSNumber numberWithLongLong:value];
    () = msg![env; this setObject:num forKey:key];
}

- (())removeObjectForKey:(id)key {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    () = msg![env; defaults removeObjectForKey:key];
}

// Apple's
// <https://developer.apple.com/documentation/foundation/nsubiquitouskeyvaluestore/1413577-dictionaryrepresentation>:
// "Returns a dictionary containing all of the key-value pairs in the
//  ubiquitous key-value store object." We back the store with
// `NSUserDefaults` (since we have no real iCloud), so the documented
// behaviour is to forward to `-[NSUserDefaults dictionaryRepresentation]`,
// which itself is documented at
// <https://developer.apple.com/documentation/foundation/nsuserdefaults/1415919-dictionaryrepresentation>.
- (id)dictionaryRepresentation {
    let defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; defaults dictionaryRepresentation]
}

@end

};
