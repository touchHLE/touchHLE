/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFPreferences`.
//!
//! According to Apple's docs, it's not toll-free bridged to `NSUserDefaults`,
//! but we are still implementing one atop of another.

use super::cf_string::CFStringRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class};
use crate::Environment;

type CFPropertyListRef = CFTypeRef;

fn CFPreferencesCopyAppValue(
    env: &mut Environment,
    key: CFStringRef,
    app_id: CFStringRef,
) -> CFPropertyListRef {
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    // TODO: handle other ids
    assert!(msg![env; app_id isEqualToString:current_app]);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    let value: id = msg![env; user_defaults objectForKey:key];
    msg![env; value copy]
}

fn CFPreferencesSetAppValue(
    env: &mut Environment,
    key: CFStringRef,
    value: CFPropertyListRef,
    app_id: CFStringRef,
) {
    assert!(!value.is_null()); // TODO
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    // TODO: handle other ids
    assert!(msg![env; app_id isEqualToString:current_app]);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; user_defaults setObject:value forKey:key]
}

fn CFPreferencesAppSynchronize(env: &mut Environment, app_id: CFStringRef) -> bool {
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    // TODO: handle other ids
    assert!(msg![env; app_id isEqualToString:current_app]);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; user_defaults synchronize]
}

pub const kCFPreferencesCurrentApplication: &str = "kCFPreferencesCurrentApplication";

pub const CONSTANTS: ConstantExports = &[(
    "_kCFPreferencesCurrentApplication",
    HostConstant::NSString(kCFPreferencesCurrentApplication),
)];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFPreferencesCopyAppValue(_, _)),
    export_c_func!(CFPreferencesSetAppValue(_, _, _)),
    export_c_func!(CFPreferencesAppSynchronize(_)),
];
