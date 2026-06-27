/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFPreferences`.
//!
//! According to Apple's docs, it's not toll-free bridged to `NSUserDefaults`,
//! but we are still implementing one atop of another.

use super::cf_array::CFArrayRef;
use super::cf_string::CFStringRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;

type CFPropertyListRef = CFTypeRef;

/// Decide whether a CFPreferences `applicationID` argument refers to the
/// running app's preferences and so can be routed to `NSUserDefaults`.
///
/// In a real iOS sandbox a process can normally only read its own
/// preferences anyway, so we accept:
///
///   - `kCFPreferencesCurrentApplication` (the documented "use the current
///     app" sentinel),
///   - the running app's own bundle identifier (e.g. PopCap's PvZ2 calls
///     `CFPreferencesCopyAppValue(key, CFSTR("com.popcap.ios.PvZ2"))`
///     instead of going through the sentinel — appdb report #49),
///   - `NULL` (some apps pass nil as a shorthand for "current app").
///
/// Any other identifier produces a one-line warning and is treated as the
/// current app, so the call still degrades to a normal `NSUserDefaults`
/// lookup instead of asserting.
fn app_id_is_current(env: &mut Environment, app_id: CFStringRef) -> bool {
    if app_id == nil {
        return true;
    }
    let current_app = ns_string::get_static_str(env, kCFPreferencesCurrentApplication);
    if msg![env; app_id isEqualToString:current_app] {
        return true;
    }
    let app_id_str = ns_string::to_rust_string(env, app_id);
    let bundle_id = env.bundle.bundle_identifier();
    if app_id_str == bundle_id {
        return true;
    }
    log!(
        "Warning: CFPreferences called with applicationID {:?}, which is \
         neither kCFPreferencesCurrentApplication nor this app's bundle id \
         ({:?}); routing to NSUserDefaults anyway.",
        app_id_str,
        bundle_id,
    );
    false
}

fn CFPreferencesCopyAppValue(
    env: &mut Environment,
    key: CFStringRef,
    app_id: CFStringRef,
) -> CFPropertyListRef {
    let _ = app_id_is_current(env, app_id);
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
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    if value.is_null() {
        let _: () = msg![env; user_defaults removeObjectForKey:key];
        return;
    }
    msg![env; user_defaults setObject:value forKey:key]
}

fn CFPreferencesAppSynchronize(env: &mut Environment, app_id: CFStringRef) -> bool {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; user_defaults synchronize]
}

/// `CFPreferencesCopyKeyList(applicationID, userName, hostName) -> CFArrayRef`
///
/// Apple docs: Constructs and returns the list of all keys set in the given
/// location. In a sandboxed iOS app only the current-application domain is
/// accessible, so we route this to `NSUserDefaults` regardless of the
/// `userName`/`hostName` arguments (which are always one of the sentinel
/// constants on iOS anyway).
///
/// Returns NULL if the domain has no keys; the caller is responsible for
/// releasing the returned array.
fn CFPreferencesCopyKeyList(
    env: &mut Environment,
    app_id: CFStringRef,
    _user_name: CFStringRef,
    _host_name: CFStringRef,
) -> CFArrayRef {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    // dictionaryRepresentation merges all domains (registration, global, app).
    // allKeys on the resulting dictionary gives us every set key.
    let dict: id = msg![env; user_defaults dictionaryRepresentation];
    if dict == nil {
        return nil;
    }
    let keys: id = msg![env; dict allKeys];
    if keys == nil {
        return nil;
    }
    // Return an autoreleased copy so the caller owns a retained ref after
    // an explicit CFRetain, matching real CoreFoundation Copy semantics.
    let count: i32 = msg![env; keys count];
    if count == 0 {
        return nil;
    }
    msg![env; keys copy]
}

/// `CFPreferencesSetValue(key, value, applicationID, userName, hostName)`
///
/// Apple docs: The primitive set function — adds, modifies, or removes a
/// preference value for the specified domain.  If `value` is NULL the key
/// is removed.  On iOS the only writable domain is the current-application /
/// current-user / any-host combination, so we route all writes to
/// `NSUserDefaults` the same way `CFPreferencesSetAppValue` does.
fn CFPreferencesSetValue(
    env: &mut Environment,
    key: CFStringRef,
    value: CFPropertyListRef,
    app_id: CFStringRef,
    _user_name: CFStringRef,
    _host_name: CFStringRef,
) {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    if value.is_null() {
        let _: () = msg![env; user_defaults removeObjectForKey:key];
        return;
    }
    msg![env; user_defaults setObject:value forKey:key]
}

/// `CFPreferencesSynchronize(applicationID, userName, hostName) -> Boolean`
///
/// Apple docs: For the specified domain, writes all pending changes to
/// permanent storage and reads the latest preference data from permanent
/// storage.  We route to `NSUserDefaults synchronize` which already does
/// this for the current app.
fn CFPreferencesSynchronize(
    env: &mut Environment,
    app_id: CFStringRef,
    _user_name: CFStringRef,
    _host_name: CFStringRef,
) -> bool {
    let _ = app_id_is_current(env, app_id);
    let user_defaults: id = msg_class![env; NSUserDefaults standardUserDefaults];
    msg![env; user_defaults synchronize]
}

pub const kCFPreferencesCurrentApplication: &str = "kCFPreferencesCurrentApplication";
pub const kCFPreferencesAnyApplication: &str = "kCFPreferencesAnyApplication";
pub const kCFPreferencesAnyHost: &str = "kCFPreferencesAnyHost";
pub const kCFPreferencesCurrentHost: &str = "kCFPreferencesCurrentHost";
pub const kCFPreferencesAnyUser: &str = "kCFPreferencesAnyUser";
pub const kCFPreferencesCurrentUser: &str = "kCFPreferencesCurrentUser";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFPreferencesCurrentApplication",
        HostConstant::NSString(kCFPreferencesCurrentApplication),
    ),
    (
        "_kCFPreferencesAnyApplication",
        HostConstant::NSString(kCFPreferencesAnyApplication),
    ),
    (
        "_kCFPreferencesAnyHost",
        HostConstant::NSString(kCFPreferencesAnyHost),
    ),
    (
        "_kCFPreferencesCurrentHost",
        HostConstant::NSString(kCFPreferencesCurrentHost),
    ),
    (
        "_kCFPreferencesAnyUser",
        HostConstant::NSString(kCFPreferencesAnyUser),
    ),
    (
        "_kCFPreferencesCurrentUser",
        HostConstant::NSString(kCFPreferencesCurrentUser),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFPreferencesCopyAppValue(_, _)),
    export_c_func!(CFPreferencesSetAppValue(_, _, _)),
    export_c_func!(CFPreferencesAppSynchronize(_)),
    export_c_func!(CFPreferencesCopyKeyList(_, _, _)),
    export_c_func!(CFPreferencesSetValue(_, _, _, _, _)),
    export_c_func!(CFPreferencesSynchronize(_, _, _)),
];
