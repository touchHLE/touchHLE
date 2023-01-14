//! Virtual filesystem, or "guest filesystem".
//!
//! This lets us put files and directories where the guest app expects them to
//! be, without constraining the layout of the host filesystem.
//!
//! Currently the filesystem layout is frozen at the point of creation, so files
//! and directories can't be created, deleted, renamed or moved.
//!
//! All files in the guest filesystem have a corresponding file in the host
//! filesystem. Accessing a file requires traversing the guest filesystem's
//! directory structure to find out the host path, but after that is done, the
//! host file is accessed directly; there is no virtualization of file I/O.
//! Currently only read-only access is permitted.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum FsNode {
    File { host_path: PathBuf },
    Directory { children: HashMap<String, FsNode> },
}
impl FsNode {
    fn from_host_dir(host_path: &Path) -> Self {
        let mut children = HashMap::new();
        for entry in std::fs::read_dir(host_path).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            let host_path = entry.path();
            let name = entry.file_name().into_string().unwrap();

            if kind.is_symlink() {
                unimplemented!("Symlink: {:?}", host_path);
            } else if kind.is_file() {
                children.insert(name, FsNode::File { host_path });
            } else if kind.is_dir() {
                children.insert(name, FsNode::from_host_dir(&host_path));
            } else {
                panic!("{:?} is not a symlink, file or directory", host_path);
            }
        }
        FsNode::Directory { children }
    }

    // Convenience methods for constructing the initial filesystem layout

    fn dir() -> Self {
        FsNode::Directory {
            children: HashMap::new(),
        }
    }
    fn with_child(mut self, name: &str, child: FsNode) -> Self {
        let FsNode::Directory { ref mut children } = self else {
            panic!();
        };
        assert!(children.insert(String::from(name), child).is_none());
        self
    }
    fn file(host_path: PathBuf) -> Self {
        FsNode::File { host_path }
    }
}

/// Like [Path] but for the virtual filesystem.
#[repr(transparent)]
#[derive(Debug)]
pub struct GuestPath(str);
impl GuestPath {
    pub fn new<S: AsRef<str>>(s: &S) -> &GuestPath {
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
    pub fn join<P: AsRef<str>>(&self, path: P) -> GuestPathBuf {
        GuestPathBuf::from(format!("{}/{}", self.as_str(), path.as_ref()))
    }

    /// Get the final component of the path.
    pub fn file_name(&self) -> Option<&str> {
        // FIXME: this should do the same resolution as `std::path::file_name()`
        let (_, file_name) = self.as_str().rsplit_once('/')?;
        Some(file_name)
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
impl std::borrow::ToOwned for GuestPath {
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
fn resolve_path<'a>(path: &'a GuestPath, relative_to: Option<&'a GuestPath>) -> Vec<&'a str> {
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

    components
}

/// The type that owns the guest filesystem and provides accessors for it.
#[derive(Debug)]
pub struct Fs {
    root: FsNode,
    current_directory: GuestPathBuf,
    home_directory: GuestPathBuf,
}
impl Fs {
    /// Construct the filesystem with some pre-defined nodes (e.g. dylibs)
    /// and the contents of the guest app bundle. Returns the new filesystem and
    /// the guest path of the bundle.
    ///
    /// The `bundle_name` argument will be used as the name of the bundle
    /// directory in the guest filesystem, and must end in `.app`.
    /// This allows the host directory for the bundle to be renamed from its
    /// original name without confusing the app. Supposedly Apple does something
    /// similar when executing iOS apps on modern Macs.
    pub fn new(bundle_host_path: &Path, bundle_dir_name: String) -> (Fs, GuestPathBuf) {
        const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000000";

        let home_directory = GuestPathBuf::from(format!("/User/Applications/{}", FAKE_UUID));
        let current_directory = home_directory.clone();

        let bundle_guest_path = home_directory.join(&bundle_dir_name);

        // Some Free Software libraries are bundled with touchHLE.
        let dylibs_host_path = Path::new("dylibs");
        let usr_lib = FsNode::dir()
            .with_child(
                "libgcc_s.1.dylib",
                FsNode::file(dylibs_host_path.join("libgcc_s.1.dylib")),
            )
            .with_child(
                // symlink
                "libstdc++.6.dylib",
                FsNode::file(dylibs_host_path.join("libstdc++.6.0.4.dylib")),
            )
            .with_child(
                "libstdc++.6.0.4.dylib",
                FsNode::file(dylibs_host_path.join("libstdc++.6.0.4.dylib")),
            );

        let root = FsNode::dir()
            .with_child(
                "User",
                FsNode::dir().with_child(
                    "Applications",
                    FsNode::dir().with_child(
                        FAKE_UUID,
                        FsNode::Directory {
                            children: HashMap::from([(
                                bundle_dir_name,
                                FsNode::from_host_dir(bundle_host_path),
                            )]),
                        },
                    ),
                ),
            )
            .with_child("usr", FsNode::dir().with_child("lib", usr_lib));

        (
            Fs {
                root,
                current_directory,
                home_directory,
            },
            bundle_guest_path,
        )
    }

    /// Get the absolute path of the guest app's (sandboxed) home directory.
    pub fn home_directory(&self) -> &GuestPath {
        &self.home_directory
    }

    fn get_node(&self, path: &GuestPath) -> Option<&FsNode> {
        let mut node = &self.root;
        for component in resolve_path(path, Some(&self.current_directory)) {
            match node {
                FsNode::Directory { children } => node = children.get(component)?,
                _ => return None,
            }
        }
        Some(node)
    }

    /// Like [std::path::Path::is_file] but for the guest filesystem.
    pub fn is_file(&self, path: &GuestPath) -> bool {
        matches!(self.get_node(path), Some(FsNode::File { .. }))
    }

    /// Like [std::fs::read] but for the guest filesystem.
    pub fn read<P: AsRef<GuestPath>>(&self, path: P) -> Result<Vec<u8>, ()> {
        match self.get_node(path.as_ref()).ok_or(())? {
            FsNode::File { host_path } => std::fs::read(host_path).map_err(|_| ()),
            _ => Err(()),
        }
    }
}
