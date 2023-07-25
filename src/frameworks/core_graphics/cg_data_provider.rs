/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGDataProvider.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::{CFDataRef, CFRetain, CFTypeRef};
use crate::Environment;

pub type CGDataProviderRef = CFTypeRef;

fn CGDataProviderCopyData(env: &mut Environment, provider: CGDataProviderRef) -> CFDataRef {
    // TODO: proper copy data once we have proper CGDataProviderRef
    CFRetain(env, provider)
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CGDataProviderCopyData(_))];
