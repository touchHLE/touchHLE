/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBundle`.
//!
//! This is not even toll-free bridged to `NSBundle` in Apple's implementation,
//! but here it is the same type.

use super::cf_array::CFArrayRef;
use super::cf_string::CFStringRef;
use super::cf_url::CFURLRef;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_bundle::NSBundleHostObject;
use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class};
use crate::Environment;

pub type CFBundleRef = super::CFTypeRef;

fn CFBundleGetMainBundle(env: &mut Environment) -> CFBundleRef {
    msg_class![env; NSBundle mainBundle]
}

fn CFBundleCopyResourcesDirectoryURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle resourceURL];
    msg![env; url copy]
}

fn CFBundleCopyResourceURL(
    env: &mut Environment,
    bundle: CFBundleRef,
    resource_name: CFStringRef,
    resource_type: CFStringRef,
    sub_dir_name: CFStringRef,
) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle URLForResource:resource_name
                                          withExtension:resource_type
                                           subdirectory:sub_dir_name];
    msg![env; url copy]
}

fn CFBundleCopyBundleLocalizations(env: &mut Environment, bundle: CFBundleRef) -> CFArrayRef {
    let bundleLocalizations = env
        .objc
        .borrow_mut::<NSBundleHostObject>(bundle)
        ._bundle
        .as_ref()
        .unwrap_or(&env.bundle)
        .bundle_localizations()
        .to_owned();
    let locArray: id = msg_class![env; _touchHLE_NSMutableArray_non_retaining alloc];
    let mut localizationStrings: Vec<String> = Vec::new();
    // TODO: Fix this mess of ownership
    for localization in bundleLocalizations {
        let localizationString = localization.to_owned().as_string().unwrap().to_owned();
        let localizationPtr: id = ns_string::from_rust_string(env, localizationString.to_owned());
        () = msg![env; locArray addObject:localizationPtr];
        localizationStrings.push(localizationString);
    }
    log_dbg!(
        "CFBundleCopyBundleLocalizations({:?}) => {:?} ({})",
        bundle,
        locArray,
        localizationStrings.join(", ")
    );
    locArray
}

fn CFBundleCopyPreferredLocalizationsFromArray(
    _env: &mut Environment,
    locArray: CFArrayRef,
) -> CFArrayRef {
    // TODO: Obtain from the OS or settings the preferred language?
    // TODO: Clone array rather than return the same reference?
    let preferredLocalizations = locArray;
    log!(
        "TODO: CFBundleCopyPreferredLocalizationsFromArray({:?}) => {:?}",
        preferredLocalizations,
        preferredLocalizations
    );
    preferredLocalizations
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFBundleGetMainBundle()),
    export_c_func!(CFBundleCopyResourcesDirectoryURL(_)),
    export_c_func!(CFBundleCopyResourceURL(_, _, _, _)),
    export_c_func!(CFBundleCopyBundleLocalizations(_)),
    export_c_func!(CFBundleCopyPreferredLocalizationsFromArray(_)),
];
