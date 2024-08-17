/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFLocale`

use super::cf_allocator::CFAllocatorRef;
use super::cf_array::CFArrayRef;
use super::cf_string::CFStringRef;
use crate::dyld::FunctionExports;
use crate::frameworks::foundation::NSUInteger;
use crate::objc::id;
use crate::{export_c_func, msg, msg_class, Environment};

type CFLocaleIdentifier = CFStringRef;

fn CFLocaleCopyPreferredLanguages(env: &mut Environment) -> CFArrayRef {
    let arr = msg_class![env; NSLocale preferredLanguages];
    msg![env; arr copy]
}

fn CFLocaleCreateCanonicalLocaleIdentifierFromString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    locale_identifier: CFStringRef,
) -> CFLocaleIdentifier {
    assert!(allocator.is_null());
    let len: NSUInteger = msg![env; locale_identifier length];
    // TODO: support arbitrary locale identification strings
    assert_eq!(len, 2);
    let ns_string: id = msg_class![env; NSString alloc];
    msg![env; ns_string initWithString:locale_identifier]
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFLocaleCopyPreferredLanguages()),
    export_c_func!(CFLocaleCreateCanonicalLocaleIdentifierFromString(_, _)),
];
