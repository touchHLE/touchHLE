/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSLocale`.

use super::{ns_array, ns_string};
use crate::dyld::{ConstantExports, HostConstant};
use crate::objc::{id, objc_classes, ClassExports, HostObject};
use crate::options::Options;
use crate::Environment;
use std::ffi::CStr;

const NSLocaleCountryCode: &str = "NSLocaleCountryCode";

pub const CONSTANTS: ConstantExports = &[(
    "_NSLocaleCountryCode",
    HostConstant::NSString(NSLocaleCountryCode),
)];

#[derive(Default)]
pub struct State {
    current_locale: Option<id>,
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

    // Unfortunately Rust-SDL2 doesn't provide a wrapper for this yet.
    let languages = unsafe {
        let mut languages = Vec::new();
        let locales_raw = sdl2_sys::SDL_GetPreferredLocales();
        if !locales_raw.is_null() {
            for i in 0.. {
                let sdl2_sys::SDL_Locale { language, country } = locales_raw.offset(i).read();
                if language.is_null() && country.is_null() {
                    // Terminator
                    break;
                }

                // The country code is ignored because many iPhone OS games
                // (e.g. Super Monkey Ball and Wolfenstein RPG) don't seem to be
                // able to handle it and fall back to English, so providing it
                // does more harm than good. It's also often unhelpful anyway:
                // on macOS, the country code seems to just be the system
                // region, rather than reflecting a preference for
                // e.g. US vs UK English.
                languages.push(CStr::from_ptr(language).to_str().unwrap().to_string());
            }
            sdl2_sys::SDL_free(locales_raw.cast());
        }
        languages
    };

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
    // Unfortunately Rust-SDL2 doesn't provide a wrapper for this yet.
    let countries = unsafe {
        let mut countries = Vec::new();
        let locales_raw = sdl2_sys::SDL_GetPreferredLocales();
        if !locales_raw.is_null() {
            for i in 0.. {
                let sdl2_sys::SDL_Locale { language, country } = locales_raw.offset(i).read();
                if language.is_null() && country.is_null() {
                    // Terminator
                    break;
                }

                // country can be NULL
                if !country.is_null() {
                    countries.push(CStr::from_ptr(country).to_str().unwrap().to_string());
                }
            }
            sdl2_sys::SDL_free(locales_raw.cast());
        }
        countries
    };

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
    country_code: id,
}
impl HostObject for NSLocaleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLocale: NSObject

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
        let host_object = NSLocaleHostObject {
            country_code
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

// TODO: constructors, more accessors

- (id)objectForKey:(id)key {
    let key_str: &str = &ns_string::to_rust_string(env, key);
    match key_str {
        NSLocaleCountryCode => {
            let &NSLocaleHostObject { country_code } = env.objc.borrow(this);
            country_code
        },
        _ => unimplemented!()
    }
}

@end

};
