/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! StoreKit.
//!
//! In-app purchases are intentionally unavailable in touchHLE. The minimal
//! implementation completes StoreKit callbacks so apps do not wait forever for
//! the App Store, but it never grants products or records transactions.

use crate::objc::id;

mod sk_payment_queue;
mod sk_product;

#[derive(Default)]
pub struct State {
    payment_queue: Option<id>,
}

impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.store_kit
    }
}

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/StoreKit.framework/StoreKit",
    aliases: &[],
    class_exports: &[sk_payment_queue::CLASSES, sk_product::CLASSES],
    constant_exports: &[],
    function_exports: &[],
};
