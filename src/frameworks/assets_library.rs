/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AssetsLibrary.framework` — a minimal host implementation that exports
//! only the string constants apps reference via dyld relocations. The
//! framework was deprecated in iOS 9 in favour of `Photos.framework`, but
//! many older games still link against it for screenshot save / camera
//! roll integration.
//!
//! Apple documentation:
//! * <https://developer.apple.com/documentation/assetslibrary>
//! * `ALAsset.h`, `ALAssetsGroup.h`

use crate::dyld::{ConstantExports, HostConstant};

/// Public AssetsLibrary string constants. Spelled to match Apple's
/// headers — apps compare them by identity / `isEqualToString:` only.
pub const CONSTANTS: ConstantExports = &[
    // ALAsset property keys.
    (
        "_ALAssetPropertyType",
        HostConstant::NSString("ALAssetPropertyType"),
    ),
    (
        "_ALAssetPropertyDate",
        HostConstant::NSString("ALAssetPropertyDate"),
    ),
    (
        "_ALAssetPropertyDuration",
        HostConstant::NSString("ALAssetPropertyDuration"),
    ),
    (
        "_ALAssetPropertyOrientation",
        HostConstant::NSString("ALAssetPropertyOrientation"),
    ),
    (
        "_ALAssetPropertyLocation",
        HostConstant::NSString("ALAssetPropertyLocation"),
    ),
    (
        "_ALAssetPropertyAssetURL",
        HostConstant::NSString("ALAssetPropertyAssetURL"),
    ),
    (
        "_ALAssetPropertyRepresentations",
        HostConstant::NSString("ALAssetPropertyRepresentations"),
    ),
    (
        "_ALAssetPropertyURLs",
        HostConstant::NSString("ALAssetPropertyURLs"),
    ),
    // ALAsset type values (passed to `valueForProperty:ALAssetPropertyType`).
    (
        "_ALAssetTypePhoto",
        HostConstant::NSString("ALAssetTypePhoto"),
    ),
    (
        "_ALAssetTypeVideo",
        HostConstant::NSString("ALAssetTypeVideo"),
    ),
    (
        "_ALAssetTypeUnknown",
        HostConstant::NSString("ALAssetTypeUnknown"),
    ),
    // ALAssetsGroup property keys.
    (
        "_ALAssetsGroupPropertyName",
        HostConstant::NSString("ALAssetsGroupPropertyName"),
    ),
    (
        "_ALAssetsGroupPropertyType",
        HostConstant::NSString("ALAssetsGroupPropertyType"),
    ),
    (
        "_ALAssetsGroupPropertyPersistentID",
        HostConstant::NSString("ALAssetsGroupPropertyPersistentID"),
    ),
    (
        "_ALAssetsGroupPropertyURL",
        HostConstant::NSString("ALAssetsGroupPropertyURL"),
    ),
    // Library change notification.
    (
        "_ALAssetsLibraryChangedNotification",
        HostConstant::NSString("ALAssetsLibraryChangedNotification"),
    ),
];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/AssetsLibrary.framework/AssetsLibrary",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[],
};
