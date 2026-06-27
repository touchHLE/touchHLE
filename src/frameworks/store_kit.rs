/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! StoreKit

mod sk_payment_queue;
mod sk_product;

use crate::dyld::{ConstantExports, HostConstant};

/// Constants used by the StoreKit framework.
///
/// These are NSString-typed `extern const` symbols. Apps that link against
/// StoreKit on iOS 6+ pull them in via Mach-O symbol lookup (e.g. for use
/// as `SKStoreProductViewController` parameter dictionary keys); without
/// these stubs the linker leaves the slots NULL, so any guest-side
/// dereference (CFString equality check, `[dict objectForKey:nil]`, etc.)
/// crashes with a NULL-page read.
pub const CONSTANTS: ConstantExports = &[
    (
        "_SKStoreProductParameterITunesItemIdentifier",
        HostConstant::NSString("itemIdentifier"),
    ),
    (
        "_SKStoreProductParameterAffiliateToken",
        HostConstant::NSString("affiliateToken"),
    ),
    (
        "_SKStoreProductParameterCampaignToken",
        HostConstant::NSString("campaignToken"),
    ),
    (
        "_SKStoreProductParameterProviderToken",
        HostConstant::NSString("providerToken"),
    ),
    (
        "_SKStoreProductParameterAdvertisingPartnerToken",
        HostConstant::NSString("advertisingPartnerToken"),
    ),
    (
        "_SKErrorDomain",
        HostConstant::NSString("SKErrorDomain"),
    ),
];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/StoreKit.framework/StoreKit",
    aliases: &[],
    class_exports: &[sk_payment_queue::CLASSES, sk_product::CLASSES],
    constant_exports: &[CONSTANTS],
    function_exports: &[],
};

#[derive(Default)]
pub struct State {
    pub payment_queue: sk_payment_queue::State,
}
