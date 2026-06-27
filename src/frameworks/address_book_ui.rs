/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `AddressBookUI.framework/AddressBookUI`.
//!
//! On iOS, AddressBookUI provides `ABPeoplePickerNavigationController`,
//! `ABPersonViewController`, etc. The vast majority of touchHLE's target
//! apps don't actually open a contacts picker — they just list
//! AddressBookUI as a Mach-O dependency, often pulled in transitively by
//! a "Recommend to a friend" or "Send a tell-a-friend email" feature
//! that's never wired up at runtime.
//!
//! Without a [crate::dyld::HostDylib] entry for the path
//! `/System/Library/Frameworks/AddressBookUI.framework/AddressBookUI`,
//! touchHLE prints a `Warning: app binary depends on unimplemented or
//! missing dylib …` at startup, which can spook users into reporting
//! otherwise-fine apps as broken (e.g. HyperHLE appdb report #64,
//! Super Yum Yum Fruit Snatch).
//!
//! This stub exists so the dependency is recognized and the warning is
//! suppressed. Real ABPeoplePickerNavigationController handling is not
//! implemented; specific AddressBookUI APIs can be added here as they
//! come up.

use crate::dyld::FunctionExports;

pub const FUNCTIONS: FunctionExports = &[];
