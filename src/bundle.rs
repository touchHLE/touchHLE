/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Utilities for working with bundles. So far we are only interested in the
//! application bundle.
//!
//! Relevant Apple documentation:
//! * [Bundle Programming Guide](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/Introduction/Introduction.html)
//!   * [Anatomy of an iOS Application Bundle](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html)
//! * [Bundle Resources](https://developer.apple.com/documentation/bundleresources?language=objc)

use crate::fs::{BundleData, Fs, GuestPath, GuestPathBuf};
use plist::dictionary::Dictionary;
use plist::Value;
use std::io::Cursor;

#[derive(Debug)]
pub struct Bundle {
    path: GuestPathBuf,
    plist: Dictionary,
}

impl Bundle {
    pub fn new_bundle_and_fs_from_host_path(
        mut bundle_data: BundleData,
    ) -> Result<(Bundle, Fs), String> {
        let plist_bytes = bundle_data.read_plist()?;

        let plist = Value::from_reader(Cursor::new(plist_bytes))
            .map_err(|_| "Could not deserialize plist data".to_string())?;

        let plist = plist
            .into_dictionary()
            .ok_or_else(|| "plist root value is not a dictionary".to_string())?;

        let bundle_name = plist["CFBundleName"].as_string().unwrap();
        let bundle_id = plist["CFBundleIdentifier"].as_string().unwrap();

        let (fs, guest_path) = Fs::new(bundle_data, format!("{bundle_name}.app"), bundle_id);

        let bundle = Bundle {
            path: guest_path,
            plist,
        };

        Ok((bundle, fs))
    }

    pub fn bundle_path(&self) -> &GuestPath {
        &self.path
    }

    pub fn bundle_identifier(&self) -> &str {
        self.plist["CFBundleIdentifier"].as_string().unwrap()
    }

    pub fn display_name(&self) -> &str {
        self.plist["CFBundleDisplayName"].as_string().unwrap()
    }

    pub fn executable_path(&self) -> GuestPathBuf {
        // FIXME: Is this key optional? All iPhone apps seem to have it.
        self.path
            .join(self.plist["CFBundleExecutable"].as_string().unwrap())
    }

    pub fn launch_image_path(&self) -> GuestPathBuf {
        if let Some(base_name) = self.plist.get("UILaunchImageFile") {
            self.path
                .join(format!("{}.png", base_name.as_string().unwrap()))
        } else {
            self.path.join("Default.png") // not guaranteed to exist!
        }
    }

    pub fn icon_path(&self) -> GuestPathBuf {
        if let Some(filename) = self.plist.get("CFBundleIconFile") {
            self.path.join(filename.as_string().unwrap())
        } else {
            self.path.join("Icon.png")
        }
    }

    pub fn main_nib_file_path(&self) -> GuestPathBuf {
        // FIXME: There might not be a main nib file, or it might be localised
        // and have multiple paths. This method should definitely be removed
        // eventually.
        self.path.join(format!(
            "{}.nib",
            self.plist["NSMainNibFile"].as_string().unwrap(),
        ))
    }
}
