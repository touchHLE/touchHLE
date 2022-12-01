//! Utilities for working with bundles. So far we are only interested in the
//! application bundle.
//!
//! Relevant Apple documentation:
//! * [Bundle Programming Guide](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/Introduction/Introduction.html)
//!   * [Anatomy of an iOS Application Bundle](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html)
//! * [Bundle Resources](https://developer.apple.com/documentation/bundleresources?language=objc)

use plist::dictionary::Dictionary;
use plist::Value;
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Bundle {
    path: PathBuf,
    plist: Dictionary,
}

impl Bundle {
    pub fn from_host_path(path: PathBuf) -> Result<Bundle, &'static str> {
        if !path.is_dir() {
            return Err("Bundle path is not a directory");
        }

        let plist_path = path.join("Info.plist");

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

        Ok(Bundle { path, plist })
    }

    pub fn display_name(&self) -> &str {
        self.plist["CFBundleDisplayName"].as_string().unwrap()
    }

    pub fn launch_image_path(&self) -> PathBuf {
        if let Some(base_name) = self.plist.get("UILaunchImageFile") {
            self.path
                .join(&format!("{}.png", base_name.as_string().unwrap()))
        } else {
            self.path.join("Default.png")
        }
    }

    pub fn icon_path(&self) -> PathBuf {
        self.path
            .join(self.plist["CFBundleIconFile"].as_string().unwrap())
    }
}
