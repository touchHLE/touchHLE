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
use crate::image::Image;
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

        let bundle_name = format!(
            "{}.app",
            if let Some(canonical) = plist.get("CFBundleName") {
                canonical.as_string().unwrap()
            } else {
                bundle_data.bundle_name()
            }
        );
        let bundle_id = plist["CFBundleIdentifier"].as_string().unwrap();

        let (fs, guest_path) = Fs::new(bundle_data, bundle_name, bundle_id);

        let bundle = Bundle {
            path: guest_path,
            plist,
        };

        Ok((bundle, fs))
    }

    /// Create a fake bundle (see [crate::Environment::new_without_app]).
    pub fn new_fake_bundle() -> Bundle {
        Bundle {
            path: GuestPathBuf::from(String::new()),
            plist: Dictionary::new(),
        }
    }

    pub fn bundle_path(&self) -> &GuestPath {
        &self.path
    }

    pub fn bundle_identifier(&self) -> &str {
        self.plist["CFBundleIdentifier"].as_string().unwrap()
    }

    pub fn bundle_version(&self) -> &str {
        self.plist["CFBundleVersion"].as_string().unwrap()
    }

    pub fn bundle_localizations(&self) -> &Vec<Value> {
        self.plist["CFBundleLocalizations"].as_array().unwrap()
    }

    /// Canonical name for the bundle according to Info.plist
    pub fn canonical_bundle_name(&self) -> Option<&str> {
        self.plist
            .get("CFBundleName")
            .map(|name| name.as_string().unwrap())
    }

    /// Name for the bundle, either the canonical name or, if there isn't one,
    /// the name this bundle has in the filesystem.
    pub fn bundle_name(&self) -> &str {
        self.path.file_name().unwrap().strip_suffix(".app").unwrap()
    }

    pub fn display_name(&self) -> &str {
        self.plist["CFBundleDisplayName"].as_string().unwrap()
    }

    pub fn minimum_os_version(&self) -> Option<&str> {
        self.plist
            .get("MinimumOSVersion")
            .map(|v| v.as_string().unwrap())
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

    fn icon_path(&self) -> GuestPathBuf {
        if let Some(filename) = self.plist.get("CFBundleIconFile") {
            if filename
                .as_string()
                .unwrap()
                .to_lowercase()
                .ends_with(".png")
            {
                self.path.join(filename.as_string().unwrap())
            } else {
                let filename_with_extension = format!("{}.png", filename.as_string().unwrap());
                self.path.join(filename_with_extension)
            }
        } else {
            self.path.join("Icon.png")
        }
    }

    /// Load icon and round off its corners for display.
    pub fn load_icon(&self, fs: &Fs) -> Result<Image, String> {
        let bytes = fs
            .read(self.icon_path())
            .map_err(|_| "Could not read icon file".to_string())?;
        let mut image =
            Image::from_bytes(&bytes).map_err(|e| format!("Could not parse icon image: {}", e))?;
        // iPhone OS icons are 57px by 57px and the OS always applies a
        // 10px radius rounded corner (see e.g. documentation of
        // UIPrerenderedIcon). If the icon is larger for some reason,
        // let's scale to match.
        let corner_radius = (10.0 / 57.0) * (image.dimensions().0 as f32);
        image.round_corners(corner_radius);
        Ok(image)
    }

    pub fn main_nib_file_path(&self) -> Option<GuestPathBuf> {
        self.plist.get("NSMainNibFile").map(|filename| {
            let filename = filename.as_string().unwrap();
            // FIXME: There main nib file might be localized and have multiple
            // paths. This method should definitely be removed eventually.
            self.path.join(format!("{}.nib", filename))
        })
    }
}
