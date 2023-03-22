/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSUserDefaults`.
//!
//! References:
//! - Apple's [Preferences and Settings Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/UserDefaults/AboutPreferenceDomains/AboutPreferenceDomains.html).

use super::ns_dictionary::dict_from_keys_and_objects;
use super::ns_string;
use crate::objc::{id, msg_class, objc_classes, ClassExports};
use crate::Environment;

#[derive(Default)]
pub struct State {
    /// `NSDictionary*`
    standard_defaults: Option<id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_user_defaults
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUserDefaults: NSObject

+ (id)standardUserDefaults {
    if let Some(existing) = State::get(env).standard_defaults {
        existing
    } else {
        // TODO: Are there other default keys we need to set?
        let langs_value: id = msg_class![env; NSLocale preferredLanguages];
        let langs_key: id = ns_string::get_static_str(env, "AppleLanguages");
        let new = dict_from_keys_and_objects(env, &[(langs_key, langs_value)]);
        State::get(env).standard_defaults = Some(new);
        new
    }
}

// TODO: plist methods etc

@end

};
