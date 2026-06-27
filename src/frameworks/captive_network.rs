/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for the CaptiveNetwork bits of `SystemConfiguration.framework`.
//!
//! `CNCopyCurrentNetworkInfo()` (and friends) live inside SystemConfiguration
//! on iOS but are typically reached through their dedicated `kCNNetworkInfo*`
//! key constants. Apps that include them defensively (e.g. analytics SDKs
//! reading the SSID) just need the symbols to be non-NULL. touchHLE has no
//! Wi-Fi telemetry, so we expose the names as plain NSString constants whose
//! identity matches the canonical Apple value.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCNNetworkInfoKeySSID",
        HostConstant::NSString("SSID"),
    ),
    (
        "_kCNNetworkInfoKeySSIDData",
        HostConstant::NSString("SSIDDATA"),
    ),
    (
        "_kCNNetworkInfoKeyBSSID",
        HostConstant::NSString("BSSID"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];
