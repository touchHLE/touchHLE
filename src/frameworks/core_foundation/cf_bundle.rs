/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBundle`.
//!
//! This is not even toll-free bridged to `NSBundle` in Apple's implementation,
//! but here it is the same type.

use super::cf_url::CFURLRef;
use crate::dyld::{export_c_func, FunctionExports};
use crate::objc::{msg, msg_class};
use crate::Environment;

pub type CFBundleRef = super::CFTypeRef;

fn CFBundleGetMainBundle(env: &mut Environment) -> CFBundleRef {
    msg_class![env; NSBundle mainBundle]
}

fn CFBundleCopyResourcesDirectoryURL(env: &mut Environment, bundle: CFBundleRef) -> CFURLRef {
    let url: CFURLRef = msg![env; bundle resourceURL];
    msg![env; url copy]
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFBundleGetMainBundle()),
    export_c_func!(CFBundleCopyResourcesDirectoryURL(_)),
];
