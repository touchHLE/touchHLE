//! Utilities for working with bundles. So far we are only interested in the
//! application bundle.
//!
//! Relevant Apple documentation:
//! * [Bundle Programming Guide](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/Introduction/Introduction.html)
//!   * [Anatomy of an iOS Application Bundle](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html)
//! * [Bundle Resources](https://developer.apple.com/documentation/bundleresources?language=objc)

use crate::fs::{Fs, GuestPath, GuestPathBuf};
use plist::dictionary::Dictionary;
use plist::Value;
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Bundle {
    path: GuestPathBuf,
    plist: Dictionary,
}

impl Bundle {
    pub fn new_bundle_and_fs_from_host_path(
        host_path: PathBuf,
    ) -> Result<(Bundle, Fs), &'static str> {
        if !host_path.is_dir() {
            return Err("Bundle path is not a directory");
        }

        let plist_path = host_path.join("Info.plist");

        if !plist_path.is_file() {
            return Err("Bundle does not contain an Info.plist file");
        }

        let plist_bytes =
            std::fs::read(plist_path).map_err(|_| "Could not read Info.plist file")?;

        let plist = Value::from_reader(Cursor::new(plist_bytes))
            .map_err(|_| "Could not deserialize plist data")?;

        let plist = plist
            .into_dictionary()
            .ok_or("plist root value is not a dictionary")?;

        let bundle_name = plist["CFBundleName"].as_string().unwrap();
        let bundle_id = plist["CFBundleIdentifier"].as_string().unwrap();

        let (fs, guest_path) = Fs::new(&host_path, format!("{}.app", bundle_name), bundle_id);

        let bundle = Bundle {
            path: guest_path,
            plist,
        };

        Ok((bundle, fs))
    }

    pub fn bundle_path(&self) -> &GuestPath {
        &self.path
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
