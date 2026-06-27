/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CoreTelephony.framework/CoreTelephony`.
//!
//! Most iOS 5+ apps (and virtually every analytics/ads SDK of that era) link
//! against CoreTelephony to read the carrier name, observe radio-access-
//! technology changes, or check the country code. touchHLE has no real
//! cellular support, so we expose a deterministic "no SIM" environment via
//! [`CTTelephonyNetworkInfo`] / [`CTCarrier`] — both are real Objective-C
//! classes that return stable values, never `nil` for the info object, and
//! never crash when KVO'd. This matches what `iPod touch` and the simulator
//! actually do on real hardware.
//!
//! The string constants for `CTRadioAccessTechnology*` and the
//! `CTRadioAccessTechnologyDidChangeNotification` notification name still
//! need to resolve to non-NULL `NSString` pointers; otherwise observers fall
//! over inside `CFStringHash` the moment they subscribe.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::objc::{
    id, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// MARK: - Default carrier values
//
// On a device with no SIM card, Apple's CoreTelephony reports an essentially
// empty `CTCarrier` (Apple actually returns "Carrier" for `carrierName` on
// the simulator and pre-iOS 16; we mirror that). All the country/network
// codes are `nil`, and `allowsVOIP` is `YES` because there is no carrier
// restriction in effect.

const DEFAULT_CARRIER_NAME: &str = "Carrier";
const DEFAULT_ALLOWS_VOIP: bool = true;

// MARK: - Host objects

#[derive(Default)]
struct CTCarrierHostObject {
    /// `NSString*` — never `nil` on Apple. We retain whatever string was
    /// installed in the carrier; a default instance gets a static string.
    carrier_name: id,
    /// `NSString*` — ISO 3166-1 alpha-2 country code, `nil` if unavailable.
    iso_country_code: id,
    /// `NSString*` — three-digit Mobile Country Code, `nil` if unavailable.
    mobile_country_code: id,
    /// `NSString*` — two- or three-digit Mobile Network Code, `nil` if
    /// unavailable.
    mobile_network_code: id,
    allows_voip: bool,
}
impl HostObject for CTCarrierHostObject {}

#[derive(Default)]
struct CTTelephonyNetworkInfoHostObject {
    /// `CTCarrier*` — owned by this info object. Lazily created the first
    /// time `subscriberCellularProvider` is queried so that we don't allocate
    /// a guest object unless an app actually asks for one.
    carrier: id,
    /// Block passed to `subscriberCellularProviderDidUpdateNotifier` —
    /// retained so apps can install a block and have the info object hang on
    /// to it. We never actually fire it (there's no carrier change to
    /// observe in touchHLE), but the property must round-trip via KVC.
    update_notifier_block: id,
}
impl HostObject for CTTelephonyNetworkInfoHostObject {}

// MARK: - Constants

pub const CONSTANTS: ConstantExports = &[
    (
        "_CTRadioAccessTechnologyDidChangeNotification",
        HostConstant::NSString("CTRadioAccessTechnologyDidChange"),
    ),
    (
        "_CTRadioAccessTechnologyGPRS",
        HostConstant::NSString("CTRadioAccessTechnologyGPRS"),
    ),
    (
        "_CTRadioAccessTechnologyEdge",
        HostConstant::NSString("CTRadioAccessTechnologyEdge"),
    ),
    (
        "_CTRadioAccessTechnologyWCDMA",
        HostConstant::NSString("CTRadioAccessTechnologyWCDMA"),
    ),
    (
        "_CTRadioAccessTechnologyHSDPA",
        HostConstant::NSString("CTRadioAccessTechnologyHSDPA"),
    ),
    (
        "_CTRadioAccessTechnologyHSUPA",
        HostConstant::NSString("CTRadioAccessTechnologyHSUPA"),
    ),
    (
        "_CTRadioAccessTechnologyCDMA1x",
        HostConstant::NSString("CTRadioAccessTechnologyCDMA1x"),
    ),
    (
        "_CTRadioAccessTechnologyCDMAEVDORev0",
        HostConstant::NSString("CTRadioAccessTechnologyCDMAEVDORev0"),
    ),
    (
        "_CTRadioAccessTechnologyCDMAEVDORevA",
        HostConstant::NSString("CTRadioAccessTechnologyCDMAEVDORevA"),
    ),
    (
        "_CTRadioAccessTechnologyCDMAEVDORevB",
        HostConstant::NSString("CTRadioAccessTechnologyCDMAEVDORevB"),
    ),
    (
        "_CTRadioAccessTechnologyeHRPD",
        HostConstant::NSString("CTRadioAccessTechnologyeHRPD"),
    ),
    (
        "_CTRadioAccessTechnologyLTE",
        HostConstant::NSString("CTRadioAccessTechnologyLTE"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];

// MARK: - Classes

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// MARK: - CTCarrier

@implementation CTCarrier: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(CTCarrierHostObject {
        carrier_name: nil,
        iso_country_code: nil,
        mobile_country_code: nil,
        mobile_network_code: nil,
        allows_voip: DEFAULT_ALLOWS_VOIP,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init {
    // Install the simulator-equivalent default values. Apps that build a
    // carrier through `[[CTCarrier alloc] init]` directly get the same
    // "Carrier" / no SIM environment as the one we attach to
    // `CTTelephonyNetworkInfo`.
    let name = get_static_str(env, DEFAULT_CARRIER_NAME);
    retain(env, name);
    env.objc.borrow_mut::<CTCarrierHostObject>(this).carrier_name = name;
    this
}

- (())dealloc {
    let host = env.objc.borrow::<CTCarrierHostObject>(this);
    let n = host.carrier_name;
    let iso = host.iso_country_code;
    let mcc = host.mobile_country_code;
    let mnc = host.mobile_network_code;
    release(env, n);
    release(env, iso);
    release(env, mcc);
    release(env, mnc);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)carrierName {
    env.objc.borrow::<CTCarrierHostObject>(this).carrier_name
}

- (id)mobileCountryCode {
    env.objc.borrow::<CTCarrierHostObject>(this).mobile_country_code
}

- (id)mobileNetworkCode {
    env.objc.borrow::<CTCarrierHostObject>(this).mobile_network_code
}

- (id)isoCountryCode {
    env.objc.borrow::<CTCarrierHostObject>(this).iso_country_code
}

- (bool)allowsVOIP {
    env.objc.borrow::<CTCarrierHostObject>(this).allows_voip
}

@end

// MARK: - CTTelephonyNetworkInfo

@implementation CTTelephonyNetworkInfo: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(CTTelephonyNetworkInfoHostObject {
        carrier: nil,
        update_notifier_block: nil,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<CTTelephonyNetworkInfoHostObject>(this);
    let carrier = host.carrier;
    let block = host.update_notifier_block;
    release(env, carrier);
    release(env, block);
    env.objc.dealloc_object(this, &mut env.mem)
}

// Lazily allocate the default carrier the first time the app asks for it
// and cache it on the info object. This matches Apple's documented
// behaviour: the property is `readonly`, but the returned `CTCarrier` is the
// same object across calls until a SIM-change event fires.
- (id)subscriberCellularProvider {
    let cached = env.objc.borrow::<CTTelephonyNetworkInfoHostObject>(this).carrier;
    if cached != nil {
        return cached;
    }
    let carrier: id = msg_class![env; CTCarrier new];
    env.objc.borrow_mut::<CTTelephonyNetworkInfoHostObject>(this).carrier = carrier;
    carrier
}

// touchHLE has no cellular radio, so there is no radio-access technology
// being used. Apple documents that this returns `nil` when no cellular
// service is available — which is exactly our situation.
- (id)currentRadioAccessTechnology {
    nil
}

- (id)subscriberCellularProviderDidUpdateNotifier {
    env.objc.borrow::<CTTelephonyNetworkInfoHostObject>(this).update_notifier_block
}

- (())setSubscriberCellularProviderDidUpdateNotifier:(id)block {
    let host = env.objc.borrow_mut::<CTTelephonyNetworkInfoHostObject>(this);
    let old = host.update_notifier_block;
    if old == block {
        return;
    }
    release(env, old);
    // Blocks need to be `Block_copy`'d before retain to move them off the
    // caller's stack; in practice touchHLE's block runtime already heap-
    // allocates blocks, so a plain `retain` is sufficient here.
    retain(env, block);
    env.objc.borrow_mut::<CTTelephonyNetworkInfoHostObject>(this).update_notifier_block = block;
}

@end

};
