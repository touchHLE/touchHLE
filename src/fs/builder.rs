/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::rc::Rc;

use crate::fs::bundle::IpaFileRef;
use crate::fs::{
    BundleData, FileLocation, Fs, FsRecord, FsRecordId, FsRecordIdGenerator, FsRecordKey,
    FsRecordKind, GuestPath, GuestPathBuf, APPLICATIONS,
};
use crate::paths;

pub struct FsBuilder {
    fs: Fs,
}

impl FsBuilder {
    pub fn new() -> Self {
        Self {
            fs: Fs {
                id_generator: FsRecordIdGenerator::new(),
                records: BTreeMap::new(),
                id_to_record_key: HashMap::new(),
                working_directory: GuestPathBuf::from(String::new()),
                home_directory: GuestPathBuf::from(String::new()),
            },
        }
    }

    pub fn with_root_dir(mut self) -> Self {
        let key = FsRecordKey {
            parent_id: FsRecordId::ROOT_PARENT,
            name: "".to_string(),
        };
        let record = FsRecord {
            id: FsRecordId::ROOT,
            kind: FsRecordKind::Directory { writeable: None },
        };
        self.fs.records.insert(key.clone(), record);
        self.fs.id_to_record_key.insert(FsRecordId::ROOT, key);
        self
    }

    pub fn with_dylibs(mut self) -> Self {
        let usr_lib_id = self.find_or_make_directory(FsRecordId::ROOT, "/usr/lib");

        let dylib_mappings = [
            ("libgcc_s.1.dylib", "libgcc_s.1.dylib"),
            ("libstdc++.6.dylib", "libstdc++.6.0.9.dylib"),
            ("libstdc++.6.0.9.dylib", "libstdc++.6.0.9.dylib"),
            ("libz.1.2.3.dylib", "libz.1.2.3.dylib"),
            ("libz.1.dylib", "libz.1.2.3.dylib"),
            ("libz.dylib", "libz.1.2.3.dylib"),
            ("libz.1.1.3.dylib", "libz.1.2.3.dylib"),
        ];

        use paths::DYLIBS_DIR;
        for (name, target) in &dylib_mappings {
            self.fs.create_file_record(
                usr_lib_id,
                name.to_string(),
                FileLocation::ResourceFilePath(format!("{DYLIBS_DIR}/{target}")),
                false,
            );
        }
        self
    }

    pub fn with_app_bundle(
        mut self,
        app_bundle: BundleData,
        bundle_dir_name: String,
        bundle_id: &str,
        read_only_mode: bool,
    ) -> Self {
        const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000000";

        let directories = ["Documents", "Library", "tmp"];
        let host_path_directories = directories.map(|dir| {
            if !read_only_mode {
                let path = paths::user_data_base_path()
                    .join(paths::SANDBOX_DIR)
                    .join(bundle_id)
                    .join(dir);
                if dir == "tmp" {
                    // We clean temporary directory for current app at startup.
                    // This is no-op if directory doesn't exist.
                    match std::fs::remove_dir_all(&path) {
                        Ok(_) => {}
                        Err(e) => {
                            log_dbg!(
                                "Unable to clean tmp host folder {:?} at startup: {}",
                                path,
                                e
                            );
                        }
                    }
                }
                if let Err(e) = std::fs::create_dir_all(&path) {
                    panic!("Could not create documents directory for app at {path:?}: {e:?}");
                }
                Some(path)
            } else {
                None
            }
        });

        if !read_only_mode {
            // Special case: Some apps may create save files at
            // Library/Preferences at the start, thus presence of that
            // directory is expected
            let path = paths::user_data_base_path()
                .join(paths::SANDBOX_DIR)
                .join(bundle_id)
                .join("Library")
                .join("Preferences");
            if let Err(e) = std::fs::create_dir_all(&path) {
                panic!("Could not create documents sub-directory for app at {path:?}: {e:?}");
            }
        }

        let app_id = self.find_or_make_directory(
            FsRecordId::ROOT,
            format!("/var/mobile/Applications/{FAKE_UUID}"),
        );

        self.mount_bundle(app_id, bundle_dir_name, app_bundle);
        for (dir, host_path) in directories.iter().zip(host_path_directories.iter()) {
            if let Some(host_path) = host_path {
                self.mount_host_dir(app_id, dir.to_string(), host_path, true);
            }
        }

        self.fs.home_directory = APPLICATIONS.join(FAKE_UUID);
        self
    }

    pub fn finalize(mut self) -> Fs {
        if self.has_root() {
            self.fs.working_directory = GuestPathBuf::from("/".to_string());
        }
        self.fs
    }

    fn has_root(&self) -> bool {
        self.fs.records.contains_key(&FsRecordKey {
            parent_id: FsRecordId::ROOT_PARENT,
            name: String::new(),
        })
    }

    fn find_or_make_directory<S: AsRef<str>>(
        &mut self,
        parent_id: FsRecordId,
        path: S,
    ) -> FsRecordId {
        let mut current_parent = parent_id;

        for part in path.as_ref().split('/').filter(|c| !c.is_empty()) {
            assert_ne!(part, "..", "unexpected .. in path: {}", path.as_ref());

            let key = FsRecordKey {
                parent_id: current_parent,
                name: part.to_string(),
            };

            if let Some(existing) = self.fs.records.get(&key) {
                if existing.is_dir() {
                    current_parent = existing.id;
                    continue;
                } else {
                    panic!("Expected directory, got {existing:?}");
                }
            }

            current_parent = self
                .fs
                .create_dir_record(current_parent, part.to_string(), None);
        }

        current_parent
    }

    fn mount_host_dir(
        &mut self,
        parent_id: FsRecordId,
        name: String,
        host_path: &Path,
        writeable: bool,
    ) {
        let parent_id = self.fs.create_dir_record(
            parent_id,
            name,
            match writeable {
                true => Some(host_path.to_owned()),
                false => None,
            },
        );
        for entry in std::fs::read_dir(host_path).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            let host_path = entry.path();
            let name = entry.file_name().into_string().unwrap();

            // There is no support for symlinks within the virtual filesystem,
            // but symlinks aren't uncommon in app bundles, so we treat a
            // symlink as if it were a copy of the file it points to.
            let kind = if kind.is_symlink() {
                std::fs::metadata(&host_path).unwrap().file_type()
            } else {
                kind
            };

            if kind.is_file() {
                self.fs.create_file_record(
                    parent_id,
                    name,
                    FileLocation::Path(host_path),
                    writeable,
                );
            } else if kind.is_dir() {
                self.mount_host_dir(parent_id, name, &host_path, writeable);
            } else {
                panic!("{host_path:?} is not a symlink, file or directory");
            }
        }
    }

    fn mount_bundle(
        &mut self,
        parent_id: FsRecordId,
        bundle_dir_name: String,
        app_bundle: BundleData,
    ) {
        match app_bundle {
            BundleData::HostDirectory(path) => {
                self.mount_host_dir(parent_id, bundle_dir_name, &path, false)
            }
            BundleData::Zip { zip, bundle_path } => {
                let archive = Rc::new(RefCell::new(zip));
                let archive_cache = Rc::new(RefCell::new(HashMap::new()));
                let metadata_map = Rc::new(RefCell::new(HashMap::new()));

                let mut archive_guard = (*archive).borrow_mut();

                let parent_id = self.fs.create_dir_record(parent_id, bundle_dir_name, None);
                for i in 0..archive_guard.len() {
                    let file = archive_guard.by_index(i).unwrap(); // TODO: report IO error?
                    let name = file.name();
                    if let Some(path) = name.strip_prefix(&bundle_path) {
                        let path = GuestPath::new(path);
                        if file.is_dir() {
                            self.find_or_make_directory(parent_id, path);
                        } else {
                            let (parent_name, file_name) = path.parent_and_file_name().unwrap();
                            assert_ne!(file_name, "..", "unexpected .. in path: {path:?}");
                            let dir = self.find_or_make_directory(parent_id, parent_name);
                            self.fs.create_file_record(
                                dir,
                                file_name.to_string(),
                                FileLocation::IpaFileRef(IpaFileRef {
                                    archive: archive.clone(),
                                    archive_files_cache: archive_cache.clone(),
                                    metadata_map: metadata_map.clone(),
                                    index: i,
                                }),
                                false,
                            );
                        }
                    }
                }
            }
        }
    }
}
