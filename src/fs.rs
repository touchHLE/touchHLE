/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Virtual filesystem, or "guest filesystem".
//!
//! This lets us put files and directories where the guest app expects them to
//! be, without constraining the layout of the host filesystem.
//!
//! Most of the filesystem is frozen at the point of creation and can't be
//! modified. The exception is the writeable parts of the app's sandboxed home
//! directory (`Documents` etc).
//!
//! All files in the guest filesystem must have a corresponding file in the host
//! filesystem, or a corresponding file inside a `.ipa` file (ZIP archive) in
//! the host filesystem. Accessing a file requires traversing the guest
//! filesystem's directory structure to find out the host path, or ZIP file
//! member. After that point, the underlying file is accessed directly; there is
//! no virtualization of file I/O.
//!
//! Directories only need a corresponding directory in the host filesystem if
//! they are writeable (i.e. if new files can be created in them).
//!
//! See also [crate::paths], which has paths for host files used by touchHLE.

mod builder;
mod bundle;

pub use builder::FsBuilder;
pub use bundle::BundleData;

use crate::fs::bundle::{IpaFile, IpaFileRef};
use crate::paths;
use core::panic;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// The actual location of a file outside the virtual filesystem, e.g. a host
/// file path.
#[derive(Debug)]
enum FileLocation {
    /// Path for a normal file. Can be read or written.
    Path(PathBuf),
    /// Reference to a file inside a `.ipa` file (ZIP archive). Read only.
    IpaFileRef(IpaFileRef),
    /// Name of a resource file bundled with touchHLE. Read only.
    ResourceFilePath(String),
}

#[derive(Debug)]
pub enum FsError {
    AlreadyExist,
    InvalidParentDir,
    NonexistentParentDir,
    ReadonlyParentDir,
}

// Put well-known paths in the guest filesystem here.

/// Path of the applications directory in the guest filesystem.
pub const APPLICATIONS: &GuestPath = GuestPath::new_const("/var/mobile/Applications");

/// Like [std::path::Path] but for the virtual filesystem.
#[repr(transparent)]
#[derive(Debug)]
pub struct GuestPath(str);
impl GuestPath {
    const fn new_const(s: &str) -> &GuestPath {
        unsafe { &*(s as *const str as *const GuestPath) }
    }

    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> &GuestPath {
        unsafe { &*(s.as_ref() as *const str as *const GuestPath) }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Join a path component.
    ///
    /// This should use `AsRef<GuestPath>`, but we can't have a blanket
    /// implementation of `AsRef<GuestPath>` for all `AsRef<str>` types, so we
    /// would have to implement it for everything that can derference to `&str`.
    /// It's easier to just use `&str`.
    ///
    /// Warning! This function should only be used for internal touchHLE
    /// purposes.
    /// For Foundation case, use `[NSString stringByAppendingPathComponent:]`
    pub fn join<P: AsRef<str>>(&self, path: P) -> GuestPathBuf {
        GuestPathBuf::from(format!("{}/{}", self.as_str(), path.as_ref()))
    }

    /// Splits the path into a parent path and a file name.
    pub fn parent_and_file_name(&self) -> Option<(&GuestPath, &str)> {
        // TODO
        assert!(!self.as_str().ends_with('/'));
        // FIXME: this should do the same resolution as `std::path::file_name()`
        let (parent_name, file_name) = self.as_str().rsplit_once('/')?;
        Some((GuestPath::new(parent_name), file_name))
    }

    /// Get the final component of the path.
    pub fn file_name(&self) -> Option<&str> {
        let (_, file_name) = self.parent_and_file_name()?;
        Some(file_name)
    }

    /// Get the parent directory of the path.
    pub fn parent(&self) -> Option<&GuestPath> {
        let (parent_name, _) = self.parent_and_file_name()?;
        Some(parent_name)
    }
}
impl AsRef<GuestPath> for GuestPath {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl AsRef<str> for GuestPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<GuestPath> for str {
    fn as_ref(&self) -> &GuestPath {
        unsafe { &*(self as *const str as *const GuestPath) }
    }
}
impl ToOwned for GuestPath {
    type Owned = GuestPathBuf;

    fn to_owned(&self) -> GuestPathBuf {
        GuestPathBuf::from(self)
    }
}

/// Like [PathBuf] but for the virtual filesystem.
#[derive(Debug, Clone)]
pub struct GuestPathBuf(String);
impl From<String> for GuestPathBuf {
    fn from(string: String) -> GuestPathBuf {
        GuestPathBuf(string)
    }
}
impl From<&GuestPath> for GuestPathBuf {
    fn from(guest_path: &GuestPath) -> GuestPathBuf {
        guest_path.as_str().to_string().into()
    }
}
impl From<GuestPathBuf> for String {
    fn from(guest_path: GuestPathBuf) -> String {
        guest_path.0
    }
}
impl std::ops::Deref for GuestPathBuf {
    type Target = GuestPath;

    fn deref(&self) -> &GuestPath {
        let s: &str = &self.0;
        s.as_ref()
    }
}
impl AsRef<GuestPath> for GuestPathBuf {
    fn as_ref(&self) -> &GuestPath {
        self
    }
}
impl std::borrow::Borrow<GuestPath> for GuestPathBuf {
    fn borrow(&self) -> &GuestPath {
        self
    }
}

fn apply_path_component<'a>(components: &mut Vec<&'a str>, component: &'a str) {
    match component {
        "" => (),
        "." => (),
        ".." => {
            components.pop();
        }
        _ => components.push(component),
    }
}

/// Resolve a path so that it is absolute and has no `.`, `..` or empty
/// components. The result is a series of zero or more path components forming
/// an absolute path (e.g. `["foo", "bar"]` means `/foo/bar`).
///
/// `relative_to` is the starting point for resolving a relative path, e.g. the
/// current directory. It must be an absolute path. It is optional if `path`
/// is absolute.
pub fn resolve_path<'a>(path: &'a GuestPath, relative_to: Option<&'a GuestPath>) -> Vec<&'a str> {
    log_dbg!("Resolving {:?} relative to {:?}", path, relative_to);

    let mut components = Vec::new();

    if !path.as_str().starts_with('/') {
        let relative_to = relative_to.unwrap().as_str();
        assert!(relative_to.starts_with('/'));
        for component in relative_to.split('/') {
            apply_path_component(&mut components, component);
        }
    }

    for component in path.as_str().split('/') {
        apply_path_component(&mut components, component);
    }

    log_dbg!("=> {:?}", components);

    components
}

/// Like [std::fs::OpenOptions] but for the guest filesystem.
/// TODO: `create_new`.
#[derive(Debug)]
pub struct GuestOpenOptions {
    read: bool,
    write: bool,
    append: bool,
    create: bool,
    truncate: bool,
}
impl GuestOpenOptions {
    pub fn new() -> GuestOpenOptions {
        GuestOpenOptions {
            read: false,
            write: false,
            append: false,
            create: false,
            truncate: false,
        }
    }
    pub fn read(&mut self) -> &mut Self {
        self.read = true;
        self
    }
    pub fn write(&mut self) -> &mut Self {
        self.write = true;
        self
    }
    pub fn append(&mut self) -> &mut Self {
        self.append = true;
        self
    }
    pub fn create(&mut self) -> &mut Self {
        self.create = true;
        self
    }
    pub fn truncate(&mut self) -> &mut Self {
        self.truncate = true;
        self
    }
}

/// Handles host I/O errors by panicking. This is intended specifically for
/// opening files. The assumption is that the guest filesystem contains all the
/// information needed to tell if opening a file should succeed, so if opening
/// the file nonetheless fails, there's either a bug or the user has done
/// something wrong.
fn handle_open_err<T, E: std::fmt::Display, P: std::fmt::Debug>(
    open_result: Result<T, E>,
    host_path: P,
) -> T {
    match open_result {
        Ok(ok) => ok,
        Err(e) => panic!("Unexpected I/O failure when trying to access real path {host_path:?}: {e}. This might indicate that files needed by touchHLE are missing, or were moved while it was running."),
    }
}

/// Like [File] but for the guest filesystem.
#[derive(Debug)]
pub enum GuestFile {
    Directory,
    File(File),
    IpaBundleFile(IpaFile),
    ResourceFile(paths::ResourceFile),
    Socket,
}

impl GuestFile {
    fn from_host_file(file: File) -> GuestFile {
        GuestFile::File(file)
    }

    fn from_ipa_file(file: &IpaFileRef) -> GuestFile {
        GuestFile::IpaBundleFile(file.open())
    }

    fn from_resource_file(file: paths::ResourceFile) -> GuestFile {
        GuestFile::ResourceFile(file)
    }

    fn from_directory() -> GuestFile {
        GuestFile::Directory
    }

    pub fn sync_all(&self) -> std::io::Result<()> {
        match self {
            GuestFile::File(file) => file.sync_all(),
            GuestFile::IpaBundleFile(_) | GuestFile::ResourceFile(_) => Ok(()),
            GuestFile::Directory => {
                log!("Warning: syncing directory as a guest file.");
                Ok(())
            }
            GuestFile::Socket => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Sync operation not supported on socket",
            )),
        }
    }
    pub fn set_len(&self, len: u64) -> std::io::Result<()> {
        match self {
            GuestFile::File(file) => file.set_len(len),
            GuestFile::IpaBundleFile(file) => {
                panic!("Attempt to resize a read-only file: {file:?}")
            }
            GuestFile::ResourceFile(file) => {
                panic!("Attempt to resize a read-only file: {file:?}")
            }
            GuestFile::Directory => panic!("Attempt to resize a directory as a guest file"),
            _ => unimplemented!(),
        }
    }

    pub fn stream_len(&mut self) -> std::io::Result<u64> {
        // TODO: Remove if standard stream_len ever gets stabilized.
        let old_position = self.stream_position()?;
        let len = self.seek(std::io::SeekFrom::End(0))?;
        self.seek(std::io::SeekFrom::Start(old_position))?;
        Ok(len)
    }

    pub fn is_seekable(&self) -> bool {
        // Due to legacy directory iteration support, directories are seekable
        // https://stackoverflow.com/questions/65911066/what-does-lseek-mean-for-a-directory-file-descriptor
        !matches!(self, GuestFile::Socket)
    }
}

impl Read for GuestFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            GuestFile::File(file) => file.read(buf),
            GuestFile::IpaBundleFile(file) => file.read(buf),
            GuestFile::ResourceFile(file) => file.get().read(buf),
            GuestFile::Directory => panic!("Attempt to read from a directory as a guest file"),
            _ => unimplemented!(),
        }
    }
}

impl Write for GuestFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            GuestFile::File(file) => file.write(buf),
            GuestFile::IpaBundleFile(file) => {
                panic!("Attempt to write to a read-only file: {file:?}")
            }
            GuestFile::ResourceFile(file) => {
                panic!("Attempt to write to a read-only file: {file:?}")
            }
            GuestFile::Directory => panic!("Attempt to write to a directory as a guest file"),
            _ => unimplemented!(),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            GuestFile::File(file) => file.flush(),
            GuestFile::IpaBundleFile(file) => {
                panic!("Attempt to flush a read-only file: {file:?}")
            }
            GuestFile::ResourceFile(file) => {
                panic!("Attempt to flush a read-only file: {file:?}")
            }
            GuestFile::Directory => panic!("Attempt to flush a directory as a guest file"),
            _ => unimplemented!(),
        }
    }
}

impl Seek for GuestFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            GuestFile::File(file) => file.seek(pos),
            GuestFile::IpaBundleFile(file) => file.seek(pos),
            GuestFile::ResourceFile(file) => file.get().seek(pos),
            GuestFile::Directory => panic!("Attempt to seek in a directory as a guest file"),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FsRecordId(u32);

impl FsRecordId {
    const ROOT_PARENT: FsRecordId = FsRecordId(0);
    const ROOT: FsRecordId = FsRecordId(1);
}

#[derive(Debug)]
pub struct FsRecordIdGenerator(u32);

impl FsRecordIdGenerator {
    pub fn new() -> Self {
        Self(FsRecordId::ROOT.0 + 1)
    }

    pub fn next(&mut self) -> FsRecordId {
        let next_id = self.0;
        self.0 += 1;
        FsRecordId(next_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FsRecordKey {
    parent_id: FsRecordId,
    name: String,
}

#[derive(Debug)]
enum FsRecordKind {
    File {
        location: FileLocation,
        writeable: bool,
    },
    Directory {
        writeable: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub struct FsRecord {
    id: FsRecordId,
    kind: FsRecordKind,
}

impl FsRecord {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, FsRecordKind::Directory { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self.kind, FsRecordKind::File { .. })
    }
}

/// The type that owns the guest filesystem and provides accessors for it.
#[derive(Debug)]
pub struct Fs {
    id_generator: FsRecordIdGenerator,
    records: BTreeMap<FsRecordKey, FsRecord>,
    id_to_record_key: HashMap<FsRecordId, FsRecordKey>,
    working_directory: GuestPathBuf,
    home_directory: GuestPathBuf,
}

impl std::fmt::Display for Fs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Filesystem:")?;
        self.display_record(f, FsRecordId::ROOT, 0)?;
        Ok(())
    }
}

impl Fs {
    /// Construct a filesystem containing a home directory for the app, its
    /// bundle and documents, and the bundled shared libraries. Returns the new
    /// filesystem and the guest path of the bundle.
    ///
    /// The `bundle_dir_name` argument will be used as the name of the bundle
    /// directory in the guest filesystem, and must end in `.app`.
    /// This allows the host directory for the bundle to be renamed from its
    /// original name without confusing the app. Supposedly Apple does something
    /// similar when executing iOS apps on modern Macs.
    ///
    /// The `bundle_id` argument should be some value that uniquely identifies
    /// the app. This will be used to construct the host path for the app's
    /// sandbox directory, where documents can be stored. A directory will be
    /// created at that path if it does not already exist.
    ///
    /// `read_only_mode` can be used when the app won't actually be run, just
    /// just inspected (e.g. to retrieve display name and icon), so no user data
    /// directories are required and no sandbox directory will be created on the
    /// host.
    pub fn new(
        app_bundle: BundleData,
        bundle_dir_name: String,
        bundle_id: &str,
        read_only_mode: bool,
    ) -> (Fs, GuestPathBuf) {
        let fs = FsBuilder::new()
            .with_root_dir()
            .with_dylibs()
            .with_app_bundle(
                app_bundle,
                bundle_dir_name.clone(),
                bundle_id,
                read_only_mode,
            )
            .finalize();

        log_dbg!("Filesystem: {}", fs);

        let bundle_guest_path = fs.home_directory.join(&bundle_dir_name);

        assert!(fs.lookup_node(&bundle_guest_path).is_some());
        (fs, bundle_guest_path)
    }

    /// Create a fake filesystem (see [crate::Environment::new_without_app]).
    pub fn new_fake_fs() -> Fs {
        FsBuilder::new().finalize()
    }

    /// Get the absolute path of the guest app's (sandboxed) home directory.
    pub fn home_directory(&self) -> &GuestPath {
        &self.home_directory
    }

    /// Get the absolute path of the current working directory. The resulting
    /// path may be invalid if the directory was moved or deleted.
    pub fn working_directory(&self) -> &GuestPath {
        &self.working_directory
    }

    /// Attempts to change the working directory.
    pub fn change_working_directory(&mut self, new_path: &GuestPath) -> Result<&GuestPath, ()> {
        let resolved = resolve_path(new_path, Some(&self.working_directory));

        let record = self.lookup_node_inner(&resolved).ok_or(())?;
        if !record.is_dir() {
            return Err(());
        }

        let new_path = if resolved.is_empty() {
            String::from("/")
        } else {
            let mut new_path = String::with_capacity(resolved.iter().map(|c| c.len() + 1).sum());
            for component in resolved {
                new_path.push('/');
                new_path.push_str(component);
            }
            new_path
        };
        self.working_directory = GuestPathBuf::from(new_path);
        Ok(&self.working_directory)
    }

    /// [Self::lookup_node] with a pre-resolved path.
    fn lookup_node_inner(&self, resolved_path_components: &[&str]) -> Option<&FsRecord> {
        let mut parent_id = FsRecordId::ROOT;

        let (file_name, parent_components) = resolved_path_components.split_last()?;

        for &component in parent_components {
            let record = self.records.get(&FsRecordKey {
                parent_id,
                name: component.to_string(),
            })?;

            match record.kind {
                FsRecordKind::Directory { .. } => parent_id = record.id,
                _ => return None,
            }
        }

        self.records.get(&FsRecordKey {
            parent_id,
            name: file_name.to_string(),
        })
    }

    /// Get the node at a given path, if it exists.
    fn lookup_node(&self, path: &GuestPath) -> Option<&FsRecord> {
        self.lookup_node_inner(&resolve_path(path, Some(&self.working_directory)))
    }

    /// Get the parent of the node at a given path, if it exists, and return it
    /// together with the final path component. This is an alternative to
    /// [Self::lookup_node] useful when writing to a file, where it might not
    /// exist yet (but its parent directory does).
    fn lookup_parent_node(&self, path: &GuestPath) -> Option<(&FsRecord, String)> {
        let components = resolve_path(path, Some(&self.working_directory));
        let (&final_component, parent_components) = components.split_last()?;

        let parent_record = self.lookup_node_inner(parent_components)?;

        Some((parent_record, final_component.to_string()))
    }

    /// Provide an iterator over the children of a parent record
    pub fn get_children(
        &self,
        parent_id: FsRecordId,
    ) -> impl Iterator<Item = (&String, &FsRecord)> {
        self.records
            .range(
                FsRecordKey {
                    parent_id,
                    name: String::new(),
                }..FsRecordKey {
                    parent_id: FsRecordId(parent_id.0 + 1),
                    name: String::new(),
                },
            )
            .map(|(key, record)| (&key.name, record))
    }

    fn create_dir_record(
        &mut self,
        parent_id: FsRecordId,
        name: String,
        writeable: Option<PathBuf>,
    ) -> FsRecordId {
        assert!(self
            .get_fs_record(parent_id)
            .is_some_and(|record| record.is_dir()));
        let key = FsRecordKey { parent_id, name };
        let id = self.id_generator.next();
        let record = FsRecord {
            id,
            kind: FsRecordKind::Directory { writeable },
        };
        self.records.insert(key.clone(), record);
        self.id_to_record_key.insert(id, key);
        id
    }

    fn create_file_record(
        &mut self,
        parent_id: FsRecordId,
        name: String,
        location: FileLocation,
        writeable: bool,
    ) -> FsRecordId {
        assert!(self
            .get_fs_record(parent_id)
            .is_some_and(|record| record.is_dir()));
        let key = FsRecordKey { parent_id, name };
        let id = self.id_generator.next();
        let record = FsRecord {
            id,
            kind: FsRecordKind::File {
                location,
                writeable,
            },
        };
        self.records.insert(key.clone(), record);
        self.id_to_record_key.insert(id, key);
        id
    }

    fn get_fs_record(&self, id: FsRecordId) -> Option<&FsRecord> {
        let record_key = self.id_to_record_key.get(&id)?;
        self.records.get(record_key)
    }

    /// Like [std::path::Path::exists] but for the guest filesystem.
    pub fn exists(&self, path: &GuestPath) -> bool {
        self.lookup_node(path).is_some()
    }

    /// Returns access information about the file/directory at the path
    /// (exists, read, write, execute)
    pub fn access(&self, path: &GuestPath) -> (bool, bool, bool, bool) {
        match self.lookup_node(path) {
            None => (false, false, false, false),
            Some(record) => match &record.kind {
                FsRecordKind::File {
                    location: _,
                    writeable,
                } => (true, true, *writeable, false),
                FsRecordKind::Directory { writeable } => (true, true, writeable.is_some(), true),
            },
        }
    }

    /// Like [std::path::Path::is_file] but for the guest filesystem.
    pub fn is_file(&self, path: &GuestPath) -> bool {
        self.lookup_node(path)
            .is_some_and(|record| record.is_file())
    }

    /// Like [std::path::Path::is_dir] but for the guest dirsystem.
    pub fn is_dir(&self, path: &GuestPath) -> bool {
        self.lookup_node(path).is_some_and(|record| record.is_dir())
    }

    pub fn modified(&self, path: &GuestPath) -> Result<i64, ()> {
        // TODO: error handling
        let record = self.lookup_node(path).ok_or(())?;
        match &record.kind {
            FsRecordKind::File { location, .. } => match location {
                // Note: the returned time is consistent with 'Date' and 'Time'
                // of files inside IPA archive as reported by 7-zip.
                // But it can be few hours off in comparison with modification
                // time reported by NSFileModificationDate for app bundle files
                // and changes if system timezone changes and apps gets
                // re-installed!
                // This shouldn't be a big problem as we're always assuming
                // GMT in the codebase right now.
                // TODO: double check that when we support different timezones
                FileLocation::IpaFileRef(ipa_file_ref) => {
                    Ok(ipa_file_ref.get_last_modified().into())
                }
                FileLocation::Path(path) => {
                    // TODO: account for the current timezone, here it's in GMT
                    fs::metadata(path)
                        .and_then(|m| m.modified())
                        .map(|t| {
                            t.duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .try_into()
                                .unwrap()
                        })
                        .map_err(|_| ())
                }
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }

    pub fn size(&self, path: &GuestPath) -> Result<u64, ()> {
        // TODO: error handling
        let record = self.lookup_node(path).ok_or(())?;
        match &record.kind {
            FsRecordKind::File { location, .. } => match location {
                FileLocation::IpaFileRef(ipa_file_ref) => Ok(ipa_file_ref.get_size()),
                FileLocation::Path(path) => {
                    fs::metadata(path).map(|meta| meta.len()).map_err(|_| ())
                }
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }

    /// Get an iterator over the names of files/directories in a directory.
    pub fn enumerate<P: AsRef<GuestPath>>(
        &self,
        path: P,
    ) -> Result<impl Iterator<Item = &str>, ()> {
        let Some(record) = self.lookup_node(path.as_ref()) else {
            return Err(());
        };
        if !record.is_dir() {
            return Err(());
        }

        Ok(self.get_children(record.id).map(|(name, _)| name.as_str()))
    }

    /// Recursively list the paths of files/directories in a directory.
    /// The base path (`path`) is not included in the returned paths.
    pub fn enumerate_recursive<P: AsRef<GuestPath>>(
        &self,
        path: P,
    ) -> Result<Vec<GuestPathBuf>, ()> {
        let Some(root_record) = self.lookup_node(path.as_ref()) else {
            return Err(());
        };
        if !root_record.is_dir() {
            return Err(());
        }

        let mut paths = Vec::new();
        let mut stack = vec![(root_record.id, String::new())];

        while let Some((current_id, current_path)) = stack.pop() {
            for (name, record) in self.get_children(current_id) {
                let child_path = if current_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", current_path, name)
                };

                paths.push(GuestPathBuf::from(child_path.clone()));

                if record.is_dir() {
                    stack.push((record.id, child_path));
                }
            }
        }
        Ok(paths)
    }

    /// Like [std::fs::read] but for the guest filesystem.
    pub fn read<P: AsRef<GuestPath>>(&self, path: P) -> Result<Vec<u8>, ()> {
        let mut file = self.open(path.as_ref())?;
        let mut result = Vec::new();
        file.read_to_end(&mut result).map_err(|_| ())?;
        Ok(result)
    }

    /// Like [std::fs::write] but for the guest filesystem.
    pub fn write<P: AsRef<GuestPath>>(&mut self, path: P, data: &[u8]) -> Result<(), ()> {
        let mut options = GuestOpenOptions::new();
        options.write().create().truncate();
        self.open_with_options(path, options)?
            .write_all(data)
            .map_err(|_| ())
    }

    /// Like [File::open] but for the guest filesystem.
    #[allow(dead_code)]
    pub fn open<P: AsRef<GuestPath>>(&self, path: P) -> Result<GuestFile, ()> {
        // it would be nice to delegate to self.open_with_options, but
        // currently it wants a mutable reference to self
        let node = self.lookup_node(path.as_ref()).ok_or(())?;
        match &node.kind {
            FsRecordKind::File { location, .. } => match location {
                FileLocation::Path(host_path) => {
                    let host_file = handle_open_err(File::open(host_path), host_path);
                    Ok(GuestFile::from_host_file(host_file))
                }
                FileLocation::IpaFileRef(file) => Ok(GuestFile::from_ipa_file(file)),
                FileLocation::ResourceFilePath(name) => {
                    let resource_file = handle_open_err(paths::ResourceFile::open(name), name);
                    Ok(GuestFile::from_resource_file(resource_file))
                }
            },
            FsRecordKind::Directory { .. } => Err(()),
        }
    }

    pub fn rename<P: AsRef<GuestPath> + Copy>(&mut self, from: P, to: P) -> Result<(), ()> {
        let from_node = self.lookup_node(from.as_ref()).ok_or(())?;
        let from_node_id = from_node.id;
        let from_host_path = match &from_node.kind {
            FsRecordKind::File {
                location: from_location,
                writeable: from_writeable,
            } => {
                let FileLocation::Path(from_host_path) = from_location else {
                    // TODO: return EISDIR
                    return Err(());
                };
                assert!(from_writeable); // TODO: return errno
                                         // TODO: avoid copy?
                from_host_path.clone()
            }
            _ => unimplemented!(),
        };

        if self.lookup_node(to.as_ref()).is_none() {
            // In case target guest node do not exist, we need to create one
            let mut options = GuestOpenOptions::new();
            options.write().create().truncate();
            self.open_with_options(to, options)?;
        }

        let to_node = self.lookup_node(to.as_ref()).unwrap();
        let FsRecordKind::File {
            location: to_location,
            writeable: to_writeable,
        } = &to_node.kind
        else {
            // TODO: return EISDIR
            return Err(());
        };
        let FileLocation::Path(to_host_path) = to_location else {
            // TODO: return EACCES
            return Err(());
        };
        assert!(to_writeable); // TODO: return errno
        let res = fs::rename(from_host_path, to_host_path);
        if res.is_ok() {
            // Remove reference to the old from node
            let node_key = self.id_to_record_key.get(&from_node_id).ok_or(())?;

            let (parent_from, _) = self.lookup_parent_node(from.as_ref()).unwrap();
            if !parent_from.is_dir() {
                panic!("Attempted to remove child from non directory.")
            };

            self.records.remove(node_key);
            self.id_to_record_key.remove(&from_node_id);
        }
        res.map_err(|_| ())
    }

    /// Like [File::options] but for the guest filesystem.
    pub fn open_with_options<P: AsRef<GuestPath>>(
        &mut self,
        path: P,
        options: GuestOpenOptions,
    ) -> Result<GuestFile, ()> {
        let GuestOpenOptions {
            read,
            write,
            append,
            create,
            truncate,
        } = options;
        assert!((!truncate && !create) || write || append);

        let path = path.as_ref();

        let (parent_id, dir_host_path, new_filename) = {
            let (parent_node, new_filename) = self.lookup_parent_node(path).ok_or(())?;
            let FsRecordKind::Directory {
                writeable: dir_host_path,
            } = &parent_node.kind
            else {
                return Err(());
            };
            (parent_node.id, dir_host_path.clone(), new_filename)
        };

        // Open an existing file if possible
        if let Some(existing_file) = self.records.get(&FsRecordKey {
            parent_id,
            name: new_filename.clone(),
        }) {
            match &existing_file.kind {
                FsRecordKind::File {
                    location,
                    writeable,
                } => {
                    if !writeable && (append || write) {
                        log!("Warning: attempt to write to read-only file {:?}", path);
                        return Err(());
                    }
                    match location {
                        FileLocation::Path(host_path) => {
                            let file = handle_open_err(
                                File::options()
                                    .read(read)
                                    .write(write)
                                    .append(append)
                                    .create(false)
                                    .truncate(truncate)
                                    .open(host_path),
                                host_path,
                            );
                            return Ok(GuestFile::File(file));
                        }
                        FileLocation::IpaFileRef(file) => {
                            assert!(!(*writeable || append || write));
                            return Ok(GuestFile::from_ipa_file(file));
                        }
                        FileLocation::ResourceFilePath(name) => {
                            assert!(!(*writeable || append || write));
                            let resource_file =
                                handle_open_err(paths::ResourceFile::open(name), name);
                            return Ok(GuestFile::from_resource_file(resource_file));
                        }
                    }
                }
                FsRecordKind::Directory { .. } => {
                    if write {
                        return Err(());
                    } else {
                        return Ok(GuestFile::from_directory());
                    }
                }
            }
        };

        // Create a new file otherwise
        if !create {
            return Err(());
        }

        let Some(dir_host_path) = dir_host_path else {
            log!(
                "Warning: attempt to create file at path {:?}, but directory is read-only",
                path
            );
            return Err(());
        };

        for c in new_filename.chars() {
            if std::path::is_separator(c) {
                panic!("Attempt to create file at path {path:?}, but filename contains path separator character {c:?}!");
            }
        }

        let host_path = dir_host_path.join(&new_filename);

        let file = handle_open_err(
            File::options()
                .read(read)
                .write(write)
                .append(append)
                .create(create)
                .truncate(truncate)
                .open(&host_path),
            &host_path,
        );
        log_dbg!(
            "Created file at path {:?} (host path: {:?})",
            path,
            host_path
        );

        self.create_file_record(parent_id, new_filename, FileLocation::Path(host_path), true);
        Ok(GuestFile::File(file))
    }

    /// Removes a file or a directory. If the node is a directory, it must be
    /// empty.
    pub fn remove<P: AsRef<GuestPath>>(&mut self, path: P) -> Result<(), ()> {
        let path = path.as_ref();
        let (parent_node, node_name) = self.lookup_parent_node(path).ok_or(())?;

        // Parent directory is not a directory
        let FsRecordKind::Directory {
            writeable: dir_writeable,
        } = &parent_node.kind
        else {
            return Err(());
        };

        if !dir_writeable.is_some() {
            log!("Warning: attempt to delete file or directroy at path {:?}, but parent directory is read-only", path);
            return Err(());
        };

        let Some(record) = self.records.get(&FsRecordKey {
            parent_id: parent_node.id,
            name: node_name.clone(),
        }) else {
            // There is no file/directory with this name
            return Err(());
        };

        match &record.kind {
            FsRecordKind::File {
                location,
                writeable,
            } => {
                // Read-only files can't be removed. (This is probably not
                // correct, but it is safer for now.)
                if !writeable {
                    return Err(());
                }

                let host_path = match location {
                    FileLocation::Path(host_path) => host_path,
                    FileLocation::IpaFileRef(_) | FileLocation::ResourceFilePath(_) => panic!(),
                };

                handle_open_err(std::fs::remove_file(host_path), host_path);
                log_dbg!(
                    "Deleted file at path {:?} (host path: {:?})",
                    path,
                    host_path
                );
            }
            FsRecordKind::Directory { writeable } => {
                // Directory is not empty
                if self.get_children(record.id).next().is_some() {
                    return Err(());
                }

                // Read-only directories can't be removed. (This is probably not
                // correct, but it is safer for now.)
                let Some(host_path) = writeable else {
                    return Err(());
                };

                handle_open_err(std::fs::remove_dir(host_path), host_path);
                log_dbg!(
                    "Deleted directory at path {:?} (host path: {:?})",
                    path,
                    host_path
                );
            }
        }

        let parent_id = parent_node.id;
        let record_id = record.id;

        self.records.remove(&FsRecordKey {
            parent_id,
            name: node_name,
        });
        self.id_to_record_key.remove(&record_id);

        Ok(())
    }

    /// Like [std::fs::create_dir_all] but for the guest filesystem.
    pub fn create_dir_all<P: AsRef<GuestPath>>(&mut self, path: P) -> Result<(), FsError> {
        let path = path.as_ref();
        assert!(path.as_str().starts_with('/'));
        // TODO: use GuestPathBuf push() once implemented
        let mut tmp_vec = vec![""];
        let components = resolve_path(path, None);
        for component in components {
            tmp_vec.push(component);
            let res = self.create_dir(GuestPathBuf::from(tmp_vec.join("/")));
            match res {
                Ok(_) | Err(FsError::AlreadyExist) => {}
                _ => return res,
            }
        }
        Ok(())
    }

    /// Like [std::fs::create_dir] but for the guest filesystem.
    pub fn create_dir<P: AsRef<GuestPath>>(&mut self, path: P) -> Result<(), FsError> {
        let path = path.as_ref();

        let (parent_record, new_dir_name) = self
            .lookup_parent_node(path)
            .ok_or(FsError::NonexistentParentDir)?;

        // Parent directory is not a directory
        let FsRecordKind::Directory {
            writeable: dir_host_path,
        } = &parent_record.kind
        else {
            return Err(FsError::InvalidParentDir);
        };

        // There's already a file/directory with this name
        if self.records.contains_key(&FsRecordKey {
            parent_id: parent_record.id,
            name: new_dir_name.clone(),
        }) {
            return Err(FsError::AlreadyExist);
        }

        let Some(dir_host_path) = dir_host_path else {
            log!("Warning: attempt to create directory at path {:?}, but parent directory is read-only", path);
            return Err(FsError::ReadonlyParentDir);
        };

        for c in new_dir_name.chars() {
            if std::path::is_separator(c) {
                panic!("Attempt to create directory at path {path:?}, but directory name contains path separator character {c:?}!");
            }
        }

        let host_path = dir_host_path.join(&new_dir_name);

        handle_open_err(std::fs::create_dir(&host_path), &host_path);
        log_dbg!(
            "Created directory at path {:?} (host path: {:?})",
            path,
            host_path
        );
        self.create_dir_record(parent_record.id, new_dir_name, Some(host_path));
        Ok(())
    }

    fn display_record(
        &self,
        f: &mut std::fmt::Formatter,
        node_id: FsRecordId,
        depth: usize,
    ) -> std::fmt::Result {
        let record = match self.get_fs_record(node_id) {
            Some(record) => record,
            None => return Ok(()),
        };

        let indent = "  ".repeat(depth);
        let name = if node_id == FsRecordId::ROOT {
            ""
        } else {
            self.id_to_record_key
                .get(&node_id)
                .map(|key| key.name.as_str())
                .unwrap_or("?")
        };

        match &record.kind {
            FsRecordKind::Directory { writeable } => {
                let suffix = if writeable.is_some() { " (w)" } else { "" };
                writeln!(f, "{indent}{name}/{suffix}")?;

                for (_, child_record) in self.get_children(node_id) {
                    self.display_record(f, child_record.id, depth + 1)?;
                }
            }
            FsRecordKind::File { writeable, .. } => {
                let suffix = if *writeable { " (w)" } else { "" };
                writeln!(f, "{indent}{name}{suffix}")?;
            }
        }
        Ok(())
    }
}
