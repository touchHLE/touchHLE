/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! IPA file format support, allowing it to be used as part of the guest
//! filesystem.
use crate::libc::time::{calendar_date_to_timestamp, time_t, tm};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use zip::result::ZipError;
use zip::ZipArchive;

/// Represents an open app bundle, either a directory or a zip file.
pub enum BundleData {
    HostDirectory(PathBuf),
    Zip {
        zip: ZipArchive<std::fs::File>,
        /// Path to the app bundle inside the zip file.
        /// It should be `"Payload/<app name>.app"` (no trailing slash!).
        bundle_path: String,
    },
}

impl BundleData {
    fn find_bundle_path_in_archive(zip: &mut ZipArchive<std::fs::File>) -> Result<String, String> {
        for i in 0..zip.len() {
            let file = zip
                .by_index(i)
                .map_err(|e| format!("Could not open IPA archive entry: {e}"))?;
            let path = file.name();
            if let Some(name) = path
                .strip_prefix("Payload/")
                .and_then(|path| path.split_once('/'))
                .and_then(|(name, _)| name.strip_suffix(".app"))
            {
                return Ok(format!("Payload/{name}.app"));
            }
        }
        Err("no app bundle found in the IPA archive".to_string())
    }

    pub fn bundle_name(&self) -> &str {
        match self {
            BundleData::HostDirectory(bundle_path) => {
                bundle_path.file_stem().unwrap().to_str().unwrap()
            }
            BundleData::Zip { bundle_path, .. } => bundle_path
                .rsplit_once('/')
                .unwrap()
                .1
                .strip_suffix(".app")
                .unwrap(),
        }
    }

    pub fn open_host_dir(path: &Path) -> Result<BundleData, String> {
        Ok(BundleData::HostDirectory(path.to_path_buf()))
    }

    pub fn open_ipa(path: &Path) -> Result<BundleData, String> {
        let file =
            std::fs::File::open(path).map_err(|e| format!("Could not open IPA file: {e}"))?;
        let mut zip =
            ZipArchive::new(file).map_err(|e| format!("Could not open IPA archive: {e}"))?;
        let bundle_path = Self::find_bundle_path_in_archive(&mut zip)?;
        Ok(BundleData::Zip { zip, bundle_path })
    }

    pub fn open_any(path: &Path) -> Result<BundleData, String> {
        if path.is_file()
            && path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("ipa"))
                .unwrap_or(false)
        {
            Ok(Self::open_ipa(path)?)
        } else if path.is_dir() {
            Ok(Self::open_host_dir(path)?)
        } else {
            Err(format!(
                "{} is not a directory or an IPA file",
                path.display()
            ))
        }
    }

    pub fn read_plist(&mut self) -> Result<Vec<u8>, String> {
        match self {
            BundleData::HostDirectory(path) => {
                std::fs::read(path.join("Info.plist")).map_err(|e| {
                    format!("Could not read Info.plist from the app bundle directory: {e}")
                })
            }
            BundleData::Zip { zip, bundle_path } => {
                let mut file = zip
                    .by_name(&format!("{bundle_path}/Info.plist"))
                    .map_err(|e| format!("Could not open Info.plist from the IPA archive: {e}"))?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .map_err(|e| format!("Could not read Info.plist from the IPA archive: {e}"))?;
                Ok(buf)
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct ArchivedFileMetadata {
    /// Unix timestamp of file modification
    last_modified: i64,
    /// Uncompressed file size
    size: u64,
}

/// Shared (refcounted) copy of the decompressed version of a file in an IPA.
///
/// Seeking in compressed files is hard, so the simple solution is to read the
/// whole file into memory. This is shared so having multiple copies of the same
/// file open won't waste memory.
type DecompressedFile = Rc<[u8]>;

/// Represents a file inside an IPA bundle that can be opened.
#[derive(Debug)]
pub struct IpaFileRef {
    pub(super) archive: Rc<RefCell<ZipArchive<std::fs::File>>>,
    pub(super) archive_files_cache: Rc<RefCell<HashMap<usize, DecompressedFile>>>,
    pub(super) metadata_map: Rc<RefCell<HashMap<usize, ArchivedFileMetadata>>>,
    pub(super) index: usize,
}

impl IpaFileRef {
    pub fn open(&self) -> IpaFile {
        // Some games, like THPS2, use a single resource bundle file which is
        // opened each time a new game resource is being read.
        // As IPA is basically an archive, this pattern requires unzipping to be
        // done each time, which is extremely slow.
        // The solution here is to cache unzipped data in memory, which should
        // be OK as early iOS IPA files are relatively small in size.
        let mut archive_cache = (*self.archive_files_cache).borrow_mut();
        archive_cache.entry(self.index).or_insert_with(|| {
            let mut archive = (*self.archive).borrow_mut();
            let mut file = match archive.by_index(self.index) {
                Ok(file) => file,
                Err(ZipError::Io(e)) => {
                    // this is a runtime error, which we __probably__ should not
                    // bubble up to the guest
                    panic!("IO error while opening file from IPA bundle: {e}")
                }
                // anything other than IO error is a bug in the code, we should
                // always have a valid index
                Err(e) => panic!("BUG: could not open file from IPA bundle: {e}"),
            };
            let mut metadata_map = (*self.metadata_map).borrow_mut();
            assert!(!metadata_map.contains_key(&self.index));
            let modified = file.last_modified();
            // This is not the cleanest way!
            // TODO: just use `time` or `chrono` crates for time conversions
            // (this also entails a lot of refactoring in [crate::libc:time])
            let tm = tm::from(
                modified.year(),
                modified.month(),
                modified.day(),
                modified.hour(),
                modified.minute(),
                modified.second(),
            );
            let timestamp = calendar_date_to_timestamp(tm);
            metadata_map.insert(
                self.index,
                ArchivedFileMetadata {
                    last_modified: timestamp.into(),
                    size: file.size(),
                },
            );
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            Rc::from(buf)
        });
        let cached_file = Rc::clone(archive_cache.get(&self.index).unwrap());
        IpaFile {
            file: Cursor::new(cached_file),
        }
    }

    pub fn get_last_modified(&self) -> time_t {
        if !self.metadata_map.borrow().contains_key(&self.index) {
            // This will force metadata loading
            // TODO: get metadata without reading the file
            _ = self.open();
        }
        self.metadata_map
            .borrow()
            .get(&self.index)
            .unwrap()
            .last_modified
            .try_into()
            .unwrap()
    }
    pub fn get_size(&self) -> u64 {
        if !self.metadata_map.borrow().contains_key(&self.index) {
            // This will force metadata loading
            // TODO: get metadata without reading the file
            _ = self.open();
        }
        self.metadata_map.borrow().get(&self.index).unwrap().size
    }
}

/// Represents an opened file in an IPA bundle.
pub struct IpaFile {
    file: Cursor<DecompressedFile>,
}

impl Debug for IpaFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpaFile")
            .field("size", &self.file.get_ref().len())
            .finish()
    }
}

impl Read for IpaFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Seek for IpaFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}
