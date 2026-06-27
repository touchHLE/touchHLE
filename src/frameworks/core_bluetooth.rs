/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CoreBluetooth.framework/CoreBluetooth`
//!
//! CoreBluetooth is iOS's Bluetooth Low Energy stack (iOS 5+). touchHLE
//! does not currently emulate a BLE radio, but apps reference the
//! framework's exported string-constant keys at load time during normal
//! initialisation paths (e.g. registering KVO observers, building up
//! scan-option dictionaries) regardless of whether a radio is actually
//! available. Surfacing the constants prevents `dyld: missing symbol`
//! warnings and lets the affected code paths exit gracefully via
//! `CBCentralManager.state == CBManagerStateUnsupported` or by
//! treating the empty scan results as "no peripherals found".
//!
//! Per Apple's `CBAdvertisementData.h` / `CBCentralManagerConstants.h`
//! / `CBPeripheralManagerConstants.h` these are `NSString *const`
//! constants with the canonical string value matching the symbol name
//! (e.g. `kCBAdvertisementDataLocalNameKey` == `@"kCBAdvDataLocalName"`).
//!
//! References:
//! - <https://developer.apple.com/documentation/corebluetooth>
//! - `CBAdvertisementData.h` (iOS SDK)

use crate::dyld::{ConstantExports, HostConstant, HostDylib};

pub const CONSTANTS: ConstantExports = &[
    // CBAdvertisementData.h
    (
        "_CBAdvertisementDataLocalNameKey",
        HostConstant::NSString("kCBAdvDataLocalName"),
    ),
    (
        "_CBAdvertisementDataManufacturerDataKey",
        HostConstant::NSString("kCBAdvDataManufacturerData"),
    ),
    (
        "_CBAdvertisementDataServiceDataKey",
        HostConstant::NSString("kCBAdvDataServiceData"),
    ),
    (
        "_CBAdvertisementDataServiceUUIDsKey",
        HostConstant::NSString("kCBAdvDataServiceUUIDs"),
    ),
    (
        "_CBAdvertisementDataOverflowServiceUUIDsKey",
        HostConstant::NSString("kCBAdvDataOverflowServiceUUIDs"),
    ),
    (
        "_CBAdvertisementDataTxPowerLevelKey",
        HostConstant::NSString("kCBAdvDataTxPowerLevel"),
    ),
    (
        "_CBAdvertisementDataIsConnectable",
        HostConstant::NSString("kCBAdvDataIsConnectable"),
    ),
    (
        "_CBAdvertisementDataSolicitedServiceUUIDsKey",
        HostConstant::NSString("kCBAdvDataSolicitedServiceUUIDs"),
    ),
    // CBCentralManagerConstants.h
    (
        "_CBCentralManagerScanOptionAllowDuplicatesKey",
        HostConstant::NSString("kCBScanOptionAllowDuplicates"),
    ),
    (
        "_CBCentralManagerScanOptionSolicitedServiceUUIDsKey",
        HostConstant::NSString("kCBScanOptionSolicitedServiceUUIDs"),
    ),
    (
        "_CBCentralManagerOptionShowPowerAlertKey",
        HostConstant::NSString("kCBInitOptionShowPowerAlert"),
    ),
    (
        "_CBCentralManagerOptionRestoreIdentifierKey",
        HostConstant::NSString("kCBInitOptionRestoreIdentifier"),
    ),
    (
        "_CBConnectPeripheralOptionNotifyOnConnectionKey",
        HostConstant::NSString("kCBConnectOptionNotifyOnConnection"),
    ),
    (
        "_CBConnectPeripheralOptionNotifyOnDisconnectionKey",
        HostConstant::NSString("kCBConnectOptionNotifyOnDisconnection"),
    ),
    (
        "_CBConnectPeripheralOptionNotifyOnNotificationKey",
        HostConstant::NSString("kCBConnectOptionNotifyOnNotification"),
    ),
    (
        "_CBCentralManagerRestoredStatePeripheralsKey",
        HostConstant::NSString("kCBRestoredPeripherals"),
    ),
    (
        "_CBCentralManagerRestoredStateScanServicesKey",
        HostConstant::NSString("kCBRestoredScanServices"),
    ),
    (
        "_CBCentralManagerRestoredStateScanOptionsKey",
        HostConstant::NSString("kCBRestoredScanOptions"),
    ),
    // CBPeripheralManagerConstants.h
    (
        "_CBPeripheralManagerOptionShowPowerAlertKey",
        HostConstant::NSString("kCBPeripheralManagerOptionShowPowerAlert"),
    ),
    (
        "_CBPeripheralManagerOptionRestoreIdentifierKey",
        HostConstant::NSString("kCBPeripheralManagerOptionRestoreIdentifier"),
    ),
    (
        "_CBPeripheralManagerRestoredStateServicesKey",
        HostConstant::NSString("kCBRestoredServices"),
    ),
    (
        "_CBPeripheralManagerRestoredStateAdvertisementDataKey",
        HostConstant::NSString("kCBRestoredAdvertisementData"),
    ),
    // CBUUID.h
    (
        "_CBUUIDCharacteristicExtendedPropertiesString",
        HostConstant::NSString("2900"),
    ),
    (
        "_CBUUIDCharacteristicUserDescriptionString",
        HostConstant::NSString("2901"),
    ),
    (
        "_CBUUIDClientCharacteristicConfigurationString",
        HostConstant::NSString("2902"),
    ),
    (
        "_CBUUIDServerCharacteristicConfigurationString",
        HostConstant::NSString("2903"),
    ),
    (
        "_CBUUIDCharacteristicFormatString",
        HostConstant::NSString("2904"),
    ),
    (
        "_CBUUIDCharacteristicAggregateFormatString",
        HostConstant::NSString("2905"),
    ),
    // CBError.h
    (
        "_CBErrorDomain",
        HostConstant::NSString("CBErrorDomain"),
    ),
    (
        "_CBATTErrorDomain",
        HostConstant::NSString("CBATTErrorDomain"),
    ),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreBluetooth.framework/CoreBluetooth",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[],
};
