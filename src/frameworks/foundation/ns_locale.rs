/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSLocale`.

use super::{ns_array, ns_string};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_foundation::cf_locale::kCFLocaleCountryCode;
use crate::objc::{id, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr};
use crate::options::Options;
use crate::window::{get_preferred_country_codes, get_preferred_language_codes};
use crate::Environment;

const NSLocaleCountryCode: &str = "NSLocaleCountryCode";

pub const CONSTANTS: ConstantExports = &[(
    "_NSLocaleCountryCode",
    HostConstant::NSString(NSLocaleCountryCode),
)];

#[derive(Default)]
pub struct State {
    current_locale: Option<id>,
    system_locale: Option<id>,
    preferred_languages: Option<id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_locale
    }
}

/// Use `msg_class![env; NSLocale preferredLanguages]` rather than calling this
/// directly, because it may be slow and there is no caching.
fn get_preferred_languages(options: &Options) -> Vec<String> {
    if let Some(ref preferred_languages) = options.preferred_languages {
        log!("The app requested your preferred languages. {:?} will reported based on your --preferred-languages= option.", preferred_languages);
        return preferred_languages.clone();
    }

    let languages = get_preferred_language_codes();
    if languages.is_empty() {
        let lang = "en".to_string();
        log!("The app requested your preferred languages. No information could be retrieved, so {:?} (English) will be reported.", lang);
        vec![lang]
    } else {
        log!("The app requested your preferred languages. {:?} will be reported based on your system language preferences.", languages);
        languages
    }
}

fn get_preferred_countries() -> Vec<String> {
    let countries = get_preferred_country_codes();
    if countries.is_empty() {
        let country = "US".to_string();
        log!("The app requested your current locale. No country information could be retrieved, so {:?} will be reported.", country);
        vec![country]
    } else {
        log!("The app requested your current locale. {:?} will be reported based on your system region settings.", countries);
        countries
    }
}

struct NSLocaleHostObject {
    /// `NSString *`
    country_code: id,
    /// `NSString *`
    language_code: id,
}
impl HostObject for NSLocaleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLocale: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSLocaleHostObject {
        country_code: nil,
        language_code: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// The documentation isn't clear about what the format of the strings should be,
// but Super Monkey Ball does `isEqualToString:` against "fr", "es", "de", "it"
// and "ja", and its locale detection works properly, so presumably they do not
// usually have region suffixes.
+ (id)preferredLanguages {
    if let Some(existing) = State::get(env).preferred_languages {
        existing
    } else {
        let langs = get_preferred_languages(&env.options);
        let lang_ns_strings = langs.into_iter().map(|lang| ns_string::from_rust_string(env, lang)).collect();
        let new = ns_array::from_vec(env, lang_ns_strings);
        State::get(env).preferred_languages = Some(new);
        new
    }
}

+ (id)currentLocale {
    if let Some(locale) = State::get(env).current_locale {
        locale
    } else {
        let countries = get_preferred_countries();
        let country_code = ns_string::from_rust_string(env, countries[0].clone());
        let languages = get_preferred_languages(&env.options);
        let language_code = ns_string::from_rust_string(env, languages[0].clone());
        let host_object = NSLocaleHostObject {
            country_code,
            language_code,
        };
        let new_locale = env.objc.alloc_object(
            this,
            Box::new(host_object),
            &mut env.mem
        );
        State::get(env).current_locale = Some(new_locale);
        new_locale
    }
}

+ (id)systemLocale {
    if let Some(locale) = State::get(env).system_locale {
        locale
    } else {
        let host_object = NSLocaleHostObject {
            // Was confirmed on the iOS Simulator
            country_code: nil,
            language_code: nil,
        };
        let new_locale = env.objc.alloc_object(
            this,
            Box::new(host_object),
            &mut env.mem
        );
        State::get(env).system_locale = Some(new_locale);
        new_locale
    }
}

// TODO: constructors, more accessors

- (id)initWithLocaleIdentifier:(id)string { // NSString *
    let str = ns_string::to_rust_string(env, string);
    log_dbg!("[(NSLocale *){:?} initWithLocaleIdentifier:'{}']", this, str);
    retain(env, string);
    // Loosely assume 2-char lang code here
    // TODO: locale identifier parsing
    assert_eq!(2, str.len());
    assert!(str.to_lowercase().eq(&str));
    assert!(!str.contains('_') && !str.contains('-'));
    assert!(env.objc.borrow::<NSLocaleHostObject>(this).language_code == nil);
    env.objc.borrow_mut::<NSLocaleHostObject>(this).language_code = string;
    this
}

- (())dealloc {
    let &NSLocaleHostObject { country_code, language_code } = env.objc.borrow::<NSLocaleHostObject>(this);
    release(env, country_code);
    release(env, language_code);
    env.objc.dealloc_object(this, &mut env.mem)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

- (id)objectForKey:(id)key {
    let key_str: &str = &ns_string::to_rust_string(env, key);
    match key_str {
        // Note: this is not the cleanest separation between NS and CF parts
        // But it does work on the iOS Simulator
        // TODO: Define NSLocaleCountryCode _as_ kCFLocaleCountryCode
        NSLocaleCountryCode | kCFLocaleCountryCode => {
            let &NSLocaleHostObject { country_code, .. } = env.objc.borrow(this);
            country_code
        },
        _ => unimplemented!()
    }
}

@end

};
