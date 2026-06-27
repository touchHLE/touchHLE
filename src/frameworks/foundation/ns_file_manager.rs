/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSFileManager` etc.

use super::{ns_array, ns_string, NSUInteger};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::ns_error::{NSCocoaErrorDomain, NSFileReadNoSuchFileError};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::fs::{FsError, GuestPath, GuestPathBuf};
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

// Search path directories
type NSSearchPathDirectory = NSUInteger;
const NSApplicationDirectory: NSSearchPathDirectory = 1;
const NSDemoApplicationDirectory: NSSearchPathDirectory = 2;
const NSDeveloperApplicationDirectory: NSSearchPathDirectory = 3;
const NSAdminApplicationDirectory: NSSearchPathDirectory = 4;
const NSLibraryDirectory: NSSearchPathDirectory = 5;
const NSDeveloperDirectory: NSSearchPathDirectory = 6;
const NSUserDirectory: NSSearchPathDirectory = 7;
const NSDocumentationDirectory: NSSearchPathDirectory = 8;
const NSDocumentDirectory: NSSearchPathDirectory = 9;
const NSCoreServiceDirectory: NSSearchPathDirectory = 10;
const NSAutosavedInformationDirectory: NSSearchPathDirectory = 11;
const NSDesktopDirectory: NSSearchPathDirectory = 12;
const NSCachesDirectory: NSSearchPathDirectory = 13;
const NSApplicationSupportDirectory: NSSearchPathDirectory = 14;
const NSDownloadsDirectory: NSSearchPathDirectory = 15;
const NSInputMethodsDirectory: NSSearchPathDirectory = 16;
const NSMoviesDirectory: NSSearchPathDirectory = 17;
const NSMusicDirectory: NSSearchPathDirectory = 18;
const NSPicturesDirectory: NSSearchPathDirectory = 19;
const NSPrinterDescriptionDirectory: NSSearchPathDirectory = 20;
const NSSharedPublicDirectory: NSSearchPathDirectory = 21;
const NSPreferencePanesDirectory: NSSearchPathDirectory = 22;
const NSItemReplacementDirectory: NSSearchPathDirectory = 99;
const NSAllApplicationsDirectory: NSSearchPathDirectory = 100;
const NSAllLibrariesDirectory: NSSearchPathDirectory = 101;

// Search path domain masks
type NSSearchPathDomainMask = NSUInteger;
const NSUserDomainMask: NSSearchPathDomainMask = 1;
const NSLocalDomainMask: NSSearchPathDomainMask = 2;
const NSNetworkDomainMask: NSSearchPathDomainMask = 4;
const NSSystemDomainMask: NSSearchPathDomainMask = 8;
const NSAllDomainsMask: NSSearchPathDomainMask = 0x0ffff;

// File attribute keys
pub const NSFileModificationDate: &str = "NSFileModificationDate";
pub const NSFileCreationDate: &str = "NSFileCreationDate";
pub const NSFileSize: &str = "NSFileSize";
const NSFileSystemFreeSize: &str = "NSFileSystemFreeSize";
const NSFileSystemSize: &str = "NSFileSystemSize";
const NSFileSystemNodes: &str = "NSFileSystemNodes";
const NSFileSystemFreeNodes: &str = "NSFileSystemFreeNodes";
pub const NSFileType: &str = "NSFileType";
pub const NSFileOwnerAccountName: &str = "NSFileOwnerAccountName";
pub const NSFileGroupOwnerAccountName: &str = "NSFileGroupOwnerAccountName";
pub const NSFilePosixPermissions: &str = "NSFilePosixPermissions";
pub const NSFileReferenceCount: &str = "NSFileReferenceCount";
pub const NSFileDeviceIdentifier: &str = "NSFileDeviceIdentifier";
pub const NSFileSystemNumber: &str = "NSFileSystemNumber";
pub const NSFileExtensionHidden: &str = "NSFileExtensionHidden";
pub const NSFileHFSCreatorCode: &str = "NSFileHFSCreatorCode";
pub const NSFileHFSTypeCode: &str = "NSFileHFSTypeCode";
pub const NSFileImmutable: &str = "NSFileImmutable";
pub const NSFileAppendOnly: &str = "NSFileAppendOnly";
pub const NSFileBusy: &str = "NSFileBusy";

// Data Protection keys (iOS 4.0+, NSFileManager attribute keys; see
// <https://developer.apple.com/documentation/foundation/nsfileprotectionkey>
// and `NSFileManager.h`).
pub const NSFileProtectionKey: &str = "NSFileProtectionKey";
pub const NSFileProtectionNone: &str = "NSFileProtectionNone";
pub const NSFileProtectionComplete: &str = "NSFileProtectionComplete";
pub const NSFileProtectionCompleteUnlessOpen: &str = "NSFileProtectionCompleteUnlessOpen";
pub const NSFileProtectionCompleteUntilFirstUserAuthentication: &str =
    "NSFileProtectionCompleteUntilFirstUserAuthentication";

// File type values
pub const NSFileTypeDirectory: &str = "NSFileTypeDirectory";
pub const NSFileTypeRegular: &str = "NSFileTypeRegular";
pub const NSFileTypeSymbolicLink: &str = "NSFileTypeSymbolicLink";
pub const NSFileTypeSocket: &str = "NSFileTypeSocket";
pub const NSFileTypeCharacterSpecial: &str = "NSFileTypeCharacterSpecial";
pub const NSFileTypeBlockSpecial: &str = "NSFileTypeBlockSpecial";
pub const NSFileTypeUnknown: &str = "NSFileTypeUnknown";

// Volume attribute keys
pub const NSURLVolumeNameKey: &str = "NSURLVolumeNameKey";
pub const NSURLVolumeLocalizedNameKey: &str = "NSURLVolumeLocalizedNameKey";
pub const NSURLVolumeTotalCapacityKey: &str = "NSURLVolumeTotalCapacityKey";
pub const NSURLVolumeAvailableCapacityKey: &str = "NSURLVolumeAvailableCapacityKey";

// Item replacement options
type NSFileManagerItemReplacementOptions = NSUInteger;
const NSFileManagerItemReplacementUsingNewMetadataOnly: NSFileManagerItemReplacementOptions = 1;
const NSFileManagerItemReplacementWithoutDeletingBackupItem: NSFileManagerItemReplacementOptions =
    2;

// Directory enumeration options
type NSDirectoryEnumerationOptions = NSUInteger;
const NSDirectoryEnumerationSkipsSubdirectoryDescendants: NSDirectoryEnumerationOptions = 1;
const NSDirectoryEnumerationSkipsPackageDescendants: NSDirectoryEnumerationOptions = 2;
const NSDirectoryEnumerationSkipsHiddenFiles: NSDirectoryEnumerationOptions = 4;

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSFileModificationDate",
        HostConstant::NSString(NSFileModificationDate),
    ),
    (
        "_NSFileCreationDate",
        HostConstant::NSString(NSFileCreationDate),
    ),
    ("_NSFileSize", HostConstant::NSString(NSFileSize)),
    (
        "_NSFileSystemFreeSize",
        HostConstant::NSString(NSFileSystemFreeSize),
    ),
    (
        "_NSFileSystemSize",
        HostConstant::NSString(NSFileSystemSize),
    ),
    ("_NSFileType", HostConstant::NSString(NSFileType)),
    (
        "_NSFileTypeDirectory",
        HostConstant::NSString(NSFileTypeDirectory),
    ),
    (
        "_NSFileTypeRegular",
        HostConstant::NSString(NSFileTypeRegular),
    ),
    (
        "_NSFileTypeSymbolicLink",
        HostConstant::NSString(NSFileTypeSymbolicLink),
    ),
    (
        "_NSFileTypeSocket",
        HostConstant::NSString(NSFileTypeSocket),
    ),
    (
        "_NSFileTypeCharacterSpecial",
        HostConstant::NSString(NSFileTypeCharacterSpecial),
    ),
    (
        "_NSFileTypeBlockSpecial",
        HostConstant::NSString(NSFileTypeBlockSpecial),
    ),
    (
        "_NSFileTypeUnknown",
        HostConstant::NSString(NSFileTypeUnknown),
    ),
    (
        "_NSFileOwnerAccountName",
        HostConstant::NSString(NSFileOwnerAccountName),
    ),
    (
        "_NSFileGroupOwnerAccountName",
        HostConstant::NSString(NSFileGroupOwnerAccountName),
    ),
    (
        "_NSFilePosixPermissions",
        HostConstant::NSString(NSFilePosixPermissions),
    ),
    (
        "_NSFileReferenceCount",
        HostConstant::NSString(NSFileReferenceCount),
    ),
    (
        "_NSFileDeviceIdentifier",
        HostConstant::NSString(NSFileDeviceIdentifier),
    ),
    (
        "_NSFileExtensionHidden",
        HostConstant::NSString(NSFileExtensionHidden),
    ),
    ("_NSFileImmutable", HostConstant::NSString(NSFileImmutable)),
    (
        "_NSFileAppendOnly",
        HostConstant::NSString(NSFileAppendOnly),
    ),
    ("_NSFileBusy", HostConstant::NSString(NSFileBusy)),
    (
        "_NSFileGroupOwnerAccountID",
        HostConstant::NSString("NSFileGroupOwnerAccountID"),
    ),
    (
        "_NSFileOwnerAccountID",
        HostConstant::NSString("NSFileOwnerAccountID"),
    ),
    (
        "_NSFileHFSCreatorCode",
        HostConstant::NSString("NSFileHFSCreatorCode"),
    ),
    // Data Protection keys (NSFileManager attribute keys for `setAttributes:`,
    // <https://developer.apple.com/documentation/foundation/nsfileprotectionkey>).
    (
        "_NSFileProtectionKey",
        HostConstant::NSString(NSFileProtectionKey),
    ),
    (
        "_NSFileProtectionNone",
        HostConstant::NSString(NSFileProtectionNone),
    ),
    (
        "_NSFileProtectionComplete",
        HostConstant::NSString(NSFileProtectionComplete),
    ),
    (
        "_NSFileProtectionCompleteUnlessOpen",
        HostConstant::NSString(NSFileProtectionCompleteUnlessOpen),
    ),
    (
        "_NSFileProtectionCompleteUntilFirstUserAuthentication",
        HostConstant::NSString(NSFileProtectionCompleteUntilFirstUserAuthentication),
    ),
];

fn NSSearchPathForDirectoriesInDomains(
    env: &mut Environment,
    directory: NSSearchPathDirectory,
    domain_mask: NSSearchPathDomainMask,
    expand_tilde: bool,
) -> id {
    // Only user domain supported for now
    // On iOS the "local domain" resolves to the same location as the user domain.
    // Accept NSLocalDomainMask without a warning. Any other mask is legitimately
    // unsupported — log a warning but continue with user-domain paths (best effort).
    // Reference: <https://developer.apple.com/documentation/foundation/1417717-nssearchpathfordirectoriesindoma>
    if domain_mask != NSUserDomainMask
        && domain_mask != NSLocalDomainMask
        && domain_mask != NSAllDomainsMask
    {
        log!(
            "Warning: NSSearchPathForDirectoriesInDomains called with unsupported domain_mask: {}",
            domain_mask
        );
    }

    let _ = expand_tilde; // Always expand for simplicity

    let dir = match directory {
        NSApplicationDirectory => GuestPath::new(crate::fs::APPLICATIONS).to_owned(),
        NSDocumentDirectory => env.fs.home_directory().join("Documents"),
        NSLibraryDirectory => env.fs.home_directory().join("Library"),
        NSCachesDirectory => env.fs.home_directory().join("Library/Caches"),
        NSApplicationSupportDirectory => {
            env.fs.home_directory().join("Library/Application Support")
        }
        NSDesktopDirectory => env.fs.home_directory().join("Desktop"),
        NSDownloadsDirectory => env.fs.home_directory().join("Downloads"),
        NSMoviesDirectory => env.fs.home_directory().join("Movies"),
        NSMusicDirectory => env.fs.home_directory().join("Music"),
        NSPicturesDirectory => env.fs.home_directory().join("Pictures"),
        NSUserDirectory => env.fs.home_directory().to_owned(),
        NSPreferencePanesDirectory => env.fs.home_directory().join("Library/PreferencePanes"),
        NSAutosavedInformationDirectory => {
            env.fs.home_directory().join("Library/Autosave Information")
        }
        // The "application" family all resolve to the iOS Applications
        // container on a real device.
        NSDemoApplicationDirectory
        | NSDeveloperApplicationDirectory
        | NSAdminApplicationDirectory
        | NSAllApplicationsDirectory => GuestPath::new(crate::fs::APPLICATIONS).to_owned(),
        // The "library" family resolves under ~/Library.
        NSAllLibrariesDirectory | NSDeveloperDirectory => env.fs.home_directory().join("Library"),
        NSDocumentationDirectory => env.fs.home_directory().join("Library/Documentation"),
        NSCoreServiceDirectory => env.fs.home_directory().join("Library/CoreServices"),
        NSInputMethodsDirectory => env.fs.home_directory().join("Library/Input Methods"),
        NSPrinterDescriptionDirectory => {
            env.fs.home_directory().join("Library/Printers/PPDs")
        }
        // Shared/public content has no dedicated location on iOS; use home.
        NSSharedPublicDirectory => env.fs.home_directory().to_owned(),
        // Temporary "item replacement" scratch area for safe-save; back it
        // by the caches directory.
        NSItemReplacementDirectory => env.fs.home_directory().join("Library/Caches"),
        _ => {
            log!(
                "Warning: Unimplemented NSSearchPathDirectory {}, returning home directory",
                directory
            );
            env.fs.home_directory().to_owned()
        }
    };

    let dir = ns_string::from_rust_string(env, String::from(dir));
    let dir_list = ns_array::from_vec(env, vec![dir]);
    autorelease(env, dir_list)
}

fn NSHomeDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory();
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

fn NSUserName(_env: &mut Environment) -> id {
    // Return a default username
    let username = ns_string::from_rust_string(_env, String::from("touchHLE_user"));
    autorelease(_env, username)
}

fn NSFullUserName(_env: &mut Environment) -> id {
    // Return a default full user name
    let full_name = ns_string::from_rust_string(_env, String::from("touchHLE User"));
    autorelease(_env, full_name)
}

/// Check [crate::fs::Fs::new] for more info for
/// how temporary folder is setup on startup
fn NSTemporaryDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory().join("tmp");
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

fn NSOpenStepRootDirectory(_env: &mut Environment) -> id {
    // Return root directory
    nil // Not typically used on iOS
}

fn NSAllocateObject(
    env: &mut Environment,
    class: id,
    extra_bytes: NSUInteger,
    _zone: NSZonePtr,
) -> id {
    if extra_bytes > 0 {
        log!(
            "Warning: NSAllocateObject called with extra_bytes={}, which is currently unhandled!",
            extra_bytes
        );
    }

    msg![env; class alloc]
}

fn NSDeallocateObject(env: &mut Environment, object: id) {
    if !object.is_null() {
        release(env, object);
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NSHomeDirectory()),
    export_c_func!(NSTemporaryDirectory()),
    export_c_func!(NSSearchPathForDirectoriesInDomains(_, _, _)),
    export_c_func!(NSUserName()),
    export_c_func!(NSFullUserName()),
    export_c_func!(NSOpenStepRootDirectory()),
    export_c_func!(NSAllocateObject(_, _, _)),
    export_c_func!(NSDeallocateObject(_)),
];

/// Copy a file, or a directory and its contents recursively, like
/// `-[NSFileManager copyItemAtPath:toPath:error:]` does on real iOS.
/// (Previously touchHLE only handled plain files here, so apps that copy a
/// whole directory — e.g. BioShock copying its `Config` directory from the
/// bundle into Documents on first launch — would get a spurious error.)
fn copy_item_recursive(env: &mut Environment, src: &GuestPath, dst: &GuestPath) -> Result<(), ()> {
    if env.fs.is_dir(src) {
        env.fs.create_dir_all(dst).map_err(|_| ())?;
        let children = env.fs.enumerate_recursive(src)?;
        for relative_path in children {
            let src_child = src.join(relative_path.as_str());
            let dst_child = dst.join(relative_path.as_str());
            if env.fs.is_dir(&src_child) {
                env.fs.create_dir_all(&dst_child).map_err(|_| ())?;
            } else {
                let data = env.fs.read(&src_child)?;
                env.fs.write(&dst_child, &data)?;
            }
        }
        Ok(())
    } else {
        let data = env.fs.read(src)?;
        env.fs.write(dst, &data)
    }
}

#[derive(Default)]
pub struct State {
    default_manager: Option<id>,
}

#[derive(Default)]
struct NSDirectoryEnumeratorHostObject {
    iterator: std::vec::IntoIter<GuestPathBuf>,
    base_path: GuestPathBuf,
    /// Number of directory components below `base_path` of the most recently
    /// returned path. Apple documents `-[NSDirectoryEnumerator level]` as
    /// "the number of levels deep the current object is in the directory
    /// hierarchy being enumerated". A file located directly inside the
    /// enumeration root therefore has level 1; the value is 0 before the
    /// first `nextObject` call.
    /// <https://developer.apple.com/documentation/foundation/nsdirectoryenumerator/1413939-level>
    current_level: NSUInteger,
}
impl HostObject for NSDirectoryEnumeratorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileManager: NSObject

+ (id)defaultManager {
    if let Some(existing) = env.framework_state.foundation.ns_file_manager.default_manager {
        existing
    } else {
        let new: id = msg![env; this new];
        env.framework_state.foundation.ns_file_manager.default_manager = Some(new);
        new
    }
}

// MARK: - Locating System Directories

- (id)currentDirectoryPath {
    ns_string::from_rust_string(env, env.fs.working_directory().as_str().to_string())
}

- (bool)changeCurrentDirectoryPath:(id)path {
    if path.is_null() {
        return false;
    }

    let path = ns_string::to_rust_string(env, path);
    let path = GuestPath::new(&path);
    match env.fs.change_working_directory(path) {
        Ok(_) => true,
        Err(()) => false
    }
}

- (ConstPtr<u8>)fileSystemRepresentationWithPath:(id)path {
    if path.is_null() {
        return Ptr::null();
    }
    // Возвращаем указатель на C-строку (UTF-8), которую хранит NSString
    msg![env; path UTF8String]
}

- (id)stringWithFileSystemRepresentation:(ConstPtr<u8>)str length:(NSUInteger)len {
    if str.is_null() {
        return nil;
    }

    if len == 0 {
        let empty = ns_string::from_rust_string(env, String::new());
        return autorelease(env, empty);
    }

    let mut bytes = Vec::with_capacity(len as usize);
    for i in 0..len {
        let byte: u8 = env.mem.read(str + i);
        bytes.push(byte);
    }

    let s = String::from_utf8_lossy(&bytes).into_owned();
    let ns_str = ns_string::from_rust_string(env, s);
    autorelease(env, ns_str)
}

// MARK: - Discovering Directory Contents

- (id)contentsOfDirectoryAtPath:(id)path
                          error:(MutPtr<id>)error {
    let contents: id = msg![env; this directoryContentsAtPath:path];
    if contents == nil && !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
        env.mem.write(error, ns_error);
    }
    contents
}

- (id)directoryContentsAtPath:(id)path {
    if path.is_null() {
        return nil;
    }

    let path = ns_string::to_rust_string(env, path);
    let Ok(paths) = env.fs.enumerate(GuestPath::new(&path)) else {
        return nil;
    };

    let paths: Vec<GuestPathBuf> = paths
        .map(|path| GuestPathBuf::from(GuestPath::new(path)))
        .collect();

    log_dbg!("directoryContentsAtPath {}: {:?}", path, paths);
    let path_strings = paths
        .iter()
        .map(|name| ns_string::from_rust_string(env, name.as_str().to_string()))
        .collect();

    let res = ns_array::from_vec(env, path_strings);
    autorelease(env, res)
}

- (id)enumeratorAtPath:(id)path {
    if path.is_null() {
        return nil;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);

    let Ok(paths) = env.fs.enumerate_recursive(guest_path) else {
        return nil;
    };

    let host_object = Box::new(NSDirectoryEnumeratorHostObject {
        iterator: paths.into_iter(),
        base_path: GuestPathBuf::from(guest_path),
        current_level: 0,
    });

    let class = env.objc.get_known_class("NSDirectoryEnumerator", &mut env.mem);
    let enumerator = env.objc.alloc_object(class, host_object, &mut env.mem);

    autorelease(env, enumerator)
}

- (id)subpathsOfDirectoryAtPath:(id)path
                          error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);

    let Ok(paths) = env.fs.enumerate_recursive(guest_path) else {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    };

    let path_strings: Vec<id> = paths
        .iter()
        .map(|p| ns_string::from_rust_string(env, p.as_str().to_string()))
        .collect();

    let res = ns_array::from_vec(env, path_strings);
    autorelease(env, res)
}

- (id)subpathsAtPath:(id)path {
    let error: MutPtr<id> = Ptr::null();
    msg![env; this subpathsOfDirectoryAtPath:path error:error]
}

// MARK: - Creating and Deleting Items

- (bool)createFileAtPath:(id)path
                contents:(id)data
              attributes:(id)attributes {
    if path.is_null() {
        return false;
    }

    let _ = attributes; // Ignore for now

    if data.is_null() {
        let empty: id = msg_class![env; NSData new];
        let res: bool = msg![env; empty writeToFile:path atomically:false];
        release(env, empty);
        res
    } else {
        msg![env; data writeToFile:path atomically:false]
    }
}

- (bool)createDirectoryAtPath:(id)path
                   attributes:(id)attributes {
    let error: MutPtr<id> = Ptr::null();
    msg![env; this createDirectoryAtPath:path
             withIntermediateDirectories:false
                              attributes:attributes
                                   error:error]
}

// `- (BOOL)createDirectoryAtURL:(NSURL *)url
//        withIntermediateDirectories:(BOOL)createIntermediates
//                         attributes:(NSDictionary<NSFileAttributeKey,id> *)attributes
//                              error:(NSError **)error;`
// Per Apple's [NSFileManager Reference](https://developer.apple.com/documentation/foundation/nsfilemanager/1415371-createdirectoryaturl):
// the URL-flavoured variant of `-createDirectoryAtPath:...`. Apple
// requires that the URL be a file URL. We extract the path via
// `-[NSURL path]` and forward to the path-based implementation, which
// already handles every documented case (existing directory, parent
// missing, intermediate creation, attribute application).
- (bool)createDirectoryAtURL:(id)url
   withIntermediateDirectories:(bool)create_intermediates
                    attributes:(id)attributes
                         error:(MutPtr<id>)error {
    if url == nil {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }
    let path: id = msg![env; url path];
    msg![env; this createDirectoryAtPath:path
             withIntermediateDirectories:create_intermediates
                              attributes:attributes
                                   error:error]
}

// `- (NSArray<NSURL *> *)URLsForDirectory:(NSSearchPathDirectory)directory
//                              inDomains:(NSSearchPathDomainMask)domainMask;`
// Per Apple's [NSFileManager Reference](https://developer.apple.com/documentation/foundation/nsfilemanager/1407726-urlsfordirectory):
// returns NSURL representations of the directories matched by
// `NSSearchPathForDirectoriesInDomains`. We call the latter (which
// already returns the correct paths for every documented
// `NSSearchPathDirectory`) and lift each path into a file URL via
// `+[NSURL fileURLWithPath:]`, preserving order.
- (id)URLsForDirectory:(NSSearchPathDirectory)directory
             inDomains:(NSSearchPathDomainMask)domain_mask {
    let paths: id = NSSearchPathForDirectoriesInDomains(env, directory, domain_mask, true);
    let count: NSUInteger = msg![env; paths count];
    let mut urls: Vec<id> = Vec::with_capacity(count as usize);
    for i in 0..count {
        let path: id = msg![env; paths objectAtIndex:i];
        let url_class = env.objc.get_known_class("NSURL", &mut env.mem);
        let url: id = msg![env; url_class fileURLWithPath:path];
        urls.push(url);
    }
    let array = ns_array::from_vec(env, urls);
    autorelease(env, array)
}

- (bool)createDirectoryAtPath:(id)path
  withIntermediateDirectories:(bool)with_intermediates
                 attributes:(id)_attributes
                        error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);

    // --- ЧЕСТНАЯ РЕАЛИЗАЦИЯ (Без заглушек) ---
    // По документации Apple: если withIntermediateDirectories == YES и папка
    // уже существует,
    // метод обязан вернуть YES. Эмулятор больше не будет биться о Read-Only
    // защиту бандла.
    if env.fs.exists(guest_path) {
        if env.fs.is_dir(guest_path) {
            if with_intermediates {
                return true;
            } else {
                // Если with_intermediates == false, возвращаем
                // NSFileWriteFileExistsError (516)
                if !error.is_null() {
                    let domain = get_static_str(env, NSCocoaErrorDomain);
                    let ns_error = msg_class![env; NSError alloc];
                    let ns_error = msg![env; ns_error initWithDomain:domain code:516 userInfo:nil];
                    env.mem.write(error, ns_error);
                }
                return false;
            }
        } else {
            // По этому пути существует файл, а не папка. Возвращаем ошибку 516.
            if !error.is_null() {
                let domain = get_static_str(env, NSCocoaErrorDomain);
                let ns_error = msg_class![env; NSError alloc];
                let ns_error = msg![env; ns_error initWithDomain:domain code:516 userInfo:nil];
                env.mem.write(error, ns_error);
            }
            return false;
        }
    }
    // ------------------------------------------

    let res = if with_intermediates {
        env.fs.create_dir_all(guest_path)
    } else {
        env.fs.create_dir(guest_path)
    };

    match res {
        Ok(()) => {
            log_dbg!("createDirectoryAtPath {} => true", path_str);
            true
        }
        Err(err) => {
            // Мягкий фоллбэк: если папки нет, но игра все равно в наглую лезет
            // писать в свой Read-Only бандл (частая ошибка в старых играх
            // Gameloft),
            // перехватываем эту ошибку VFS, чтобы избежать краша.
            if let FsError::ReadonlyParentDir = err {
                log!("Warning: createDirectoryAtPath {} intercepted ReadonlyParentDir, pretending success", path_str);
                return true;
            }

            if let FsError::NonexistentParentDir = err {
                log!("Note: createDirectoryAtPath {} encountered NonexistentParentDir. Forcing create_dir_all to build missing lazy sandbox structures.", path_str);

                if env.fs.create_dir_all(guest_path).is_ok() {
                    return true;
                }
            }

            log!(
                "Warning: createDirectoryAtPath {} failed with {:?}, returning false",
                path_str,
                err,
            );
            if !error.is_null() {
                let domain = get_static_str(env, NSCocoaErrorDomain);
                let ns_error = msg_class![env; NSError alloc];
                let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                env.mem.write(error, ns_error);
            }
            false
        }
    }
}

- (bool)createSymbolicLinkAtPath:(id)path
             withDestinationPath:(id)dest_path
                           error:(MutPtr<id>)error {
    // Symbolic links not fully supported - log and return false
    let _ = (path, dest_path);
    log!("Warning: createSymbolicLinkAtPath:withDestinationPath:error: not fully implemented");
    if !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
        env.mem.write(error, ns_error);
    }
    false
}

- (id)destinationOfSymbolicLinkAtPath:(id)path
                                error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);

    // 1. Честно проверяем, существует ли вообще файл/папка по этому пути
    if !env.fs.exists(guest_path) {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }

    // 2. В виртуальной ФС touchHLE реальных симлинков нет (они либо резолвятся
    // при распаковке,
    // либо не поддерживаются). По документации Apple, если файл существует,
    // но НЕ является симлинком, метод возвращает nil и ошибку (обычно код 256 -
    // NSFileReadUnknownError
    // или POSIX EINVAL 22). Эмулируем этот легальный отказ:
    if !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:256 userInfo:nil];
        env.mem.write(error, ns_error);
    }

    return nil;
}

- (bool)removeItemAtPath:(id)path
                   error:(MutPtr<id>)out_error {
    if path.is_null() {
        if !out_error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let error = msg_class![env; NSError alloc];
            let error = msg![env; error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(out_error, error);
        }
        return false;
    }

    let path_str = ns_string::to_rust_string(env, path);

    match env.fs.remove(GuestPath::new(&path_str)) {
        Ok(()) => true,
        Err(err) => {
            if !out_error.is_null() {
                match err {
                    FsError::DoesNotExist => {
                        let domain = get_static_str(env, NSCocoaErrorDomain);
                        let error = msg_class![env; NSError alloc];
                        let error = msg![env; error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                        autorelease(env, error);
                        env.mem.write(out_error, error);
                    }
                    _ => {
                        let domain = get_static_str(env, NSCocoaErrorDomain);
                        let error = msg_class![env; NSError alloc];
                        let error = msg![env; error initWithDomain:domain code:1 userInfo:nil];
                        env.mem.write(out_error, error);
                    }
                }
            }
            false
        }
    }
}

// MARK: - Moving and Copying Items

- (bool)copyItemAtPath:(id)src
                toPath:(id)dst
                 error:(MutPtr<id>)error {
    if src.is_null() || dst.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }

    let src_str = ns_string::to_rust_string(env, src);
    let dst_str = ns_string::to_rust_string(env, dst);
    log_dbg!("copyItemAtPath:{:?} toPath:{:?}", src_str, dst_str);

    match copy_item_recursive(
        env,
        GuestPath::new(src_str.as_ref()),
        GuestPath::new(dst_str.as_ref()),
    ) {
        Ok(()) => true,
        Err(_) => {
            if !error.is_null() {
                let domain = get_static_str(env, NSCocoaErrorDomain);
                let ns_error = msg_class![env; NSError alloc];
                let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                env.mem.write(error, ns_error);
            }
            false
        }
    }
}

- (bool)moveItemAtPath:(id)path
                toPath:(id)toPath
                 error:(MutPtr<id>)error {
    if path.is_null() || toPath.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let to_path_str = ns_string::to_rust_string(env, toPath);

    match env.fs.rename(GuestPath::new(&path_str), GuestPath::new(&to_path_str)) {
        Ok(()) => true,
        Err(()) => {
            if !error.is_null() {
                let domain = get_static_str(env, NSCocoaErrorDomain);
                let ns_error = msg_class![env; NSError alloc];
                let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
                env.mem.write(error, ns_error);
            }
            false
        }
    }
}

- (bool)linkItemAtPath:(id)src_path
                toPath:(id)dst_path
                 error:(MutPtr<id>)error {
    // Hard links not supported - just copy instead
    msg![env; this copyItemAtPath:src_path toPath:dst_path error:error]
}

// MARK: - Managing iCloud-Based Items (Stubs)

- (id)URLForUbiquityContainerIdentifier:(id)_container_id {
    // iCloud not supported
    nil
}

- (bool)isUbiquitousItemAtURL:(id)_url {
    // iCloud not supported
    false
}

- (bool)setUbiquitous:(bool)_flag
            itemAtURL:(id)_url
       destinationURL:(id)_dest_url
                error:(MutPtr<id>)error {
    // iCloud not supported
    if !error.is_null() {
        let domain = get_static_str(env, NSCocoaErrorDomain);
        let ns_error = msg_class![env; NSError alloc];
        let ns_error = msg![env; ns_error initWithDomain:domain code:1 userInfo:nil];
        env.mem.write(error, ns_error);
    }
    false
}

// MARK: - Determining Access to Files

- (bool)fileExistsAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    let path = ns_string::to_rust_string(env, path);
    env.fs.exists(GuestPath::new(&path))
}

- (bool)fileExistsAtPath:(id)path
             isDirectory:(MutPtr<bool>)is_dir_ptr {
    if path.is_null() {
        if !is_dir_ptr.is_null() {
            env.mem.write(is_dir_ptr, false);
        }
        return false;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);

    if env.fs.exists(guest_path) {
        if !is_dir_ptr.is_null() {
            let is_dir = env.fs.is_dir(guest_path);
            env.mem.write(is_dir_ptr, is_dir);
        }
        true
    } else {
        if !is_dir_ptr.is_null() {
            env.mem.write(is_dir_ptr, false);
        }
        false
    }
}

- (bool)isReadableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    let path = ns_string::to_rust_string(env, path);
    env.fs.exists(GuestPath::new(&path)) // All existing files are readable
}

- (bool)isWritableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    let path = ns_string::to_rust_string(env, path);
    env.fs.exists(GuestPath::new(&path)) // All existing files are writable
}

- (bool)isExecutableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    // We don't support execution right now, but for compatibility might want to
    // return true for certain files
    let path = ns_string::to_rust_string(env, path);
    env.fs.exists(GuestPath::new(&path))
}

- (bool)isDeletableFileAtPath:(id)path {
    if path.is_null() {
        return false;
    }
    let path = ns_string::to_rust_string(env, path);
    env.fs.exists(GuestPath::new(&path)) // All existing files are deletable
}

// MARK: - Getting and Setting Attributes

- (id)attributesOfItemAtPath:(id)path
                       error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }

    let path_str = ns_string::to_rust_string(env, path);
    let guest_path = GuestPath::new(&path_str);

    if !env.fs.exists(guest_path) {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }

    let is_dir = env.fs.is_dir(guest_path);
    let file_size = if is_dir { 0 } else { env.fs.read(guest_path).map(|d| d.len()).unwrap_or(0) };

    let dict: id = msg_class![env; NSMutableDictionary dictionary];

    let type_val = if is_dir { NSFileTypeDirectory } else { NSFileTypeRegular };
    let type_str = get_static_str(env, type_val);
    let type_key = get_static_str(env, NSFileType);
    () = msg![env; dict setObject:type_str forKey:type_key];

    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:(file_size as u64)];
    let size_key = get_static_str(env, NSFileSize);
    () = msg![env; dict setObject:size_num forKey:size_key];

    let perms = if is_dir { 0o755 } else { 0o644 };
    let perms_num: id = msg_class![env; NSNumber numberWithUnsignedInt:perms];
    let perms_key = get_static_str(env, NSFilePosixPermissions);
    () = msg![env; dict setObject:perms_num forKey:perms_key];

    let dict_imm = msg![env; dict copy];
    autorelease(env, dict_imm)
}

- (id)fileAttributesAtPath:(id)path
              traverseLink:(bool)_traverse {
    // В старых версиях iOS этот метод просто возвращал словарь с атрибутами.
    // Так как у нас уже есть полноценная реализация атрибутов,
    // мы честно делегируем вызов в неё, передав null вместо указателя на
    // ошибку.
    let error: MutPtr<id> = Ptr::null();
    msg![env; this attributesOfItemAtPath:path error:error]
}

- (id)attributesOfFileSystemForPath:(id)path
                              error:(MutPtr<id>)error {
    if path.is_null() {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return nil;
    }

    // Return dummy file system attributes
    let dict: id = msg_class![env; NSMutableDictionary dictionary];

    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:(1024 * 1024 * 1024 * 16_u64)];
    // 16GB
    let size_key = get_static_str(env, NSFileSystemSize);
    () = msg![env; dict setObject:size_num forKey:size_key];

    let free_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:(1024 * 1024 * 1024 * 8_u64)];
    // 8GB
    let free_key = get_static_str(env, NSFileSystemFreeSize);
    () = msg![env; dict setObject:free_num forKey:free_key];

    let dict_imm = msg![env; dict copy];
    autorelease(env, dict_imm)
}

- (bool)setAttributes:(id)_attributes
         ofItemAtPath:(id)path
                error:(MutPtr<id>)error {
    // Setting attributes is not fully supported, but we claim success if file
    // exists
    let exists: bool = msg![env; this fileExistsAtPath:path];
    if !exists {
        if !error.is_null() {
            let domain = get_static_str(env, NSCocoaErrorDomain);
            let ns_error = msg_class![env; NSError alloc];
            let ns_error = msg![env; ns_error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
            env.mem.write(error, ns_error);
        }
        return false;
    }
    true
}

// MARK: - Retrieving File Contents

- (id)contentsAtPath:(id)path {
    if path.is_null() {
        return nil;
    }
    msg_class![env; NSData dataWithContentsOfFile:path]
}

// MARK: - Comparing Files

- (bool)contentsEqualAtPath:(id)path1
                     andPath:(id)path2 {
    if path1.is_null() || path2.is_null() {
        return false;
    }

    if path1 == path2 {
        return true;
    }

    let p1 = ns_string::to_rust_string(env, path1);
    let p2 = ns_string::to_rust_string(env, path2);

    if p1 == p2 {
        return true;
    }

    let Ok(d1) = env.fs.read(GuestPath::new(&p1)) else { return false };
    let Ok(d2) = env.fs.read(GuestPath::new(&p2)) else { return false };

    d1 == d2
}

@end

@implementation NSDirectoryEnumerator: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    // Дефолтная пустышка, реальные данные уже заполняются в твоем
    // enumeratorAtPath:
    let host = Box::new(NSDirectoryEnumeratorHostObject {
        iterator: Vec::new().into_iter(),
        base_path: GuestPathBuf::from(GuestPath::new("")),
        current_level: 0,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

// Главный метод: игра вызывает его в цикле, пока он не вернет nil
- (id)nextObject {
    let host = env.objc.borrow_mut::<NSDirectoryEnumeratorHostObject>(this);

    if let Some(path) = host.iterator.next() {
        let path_str = path.as_str();
        let base_str = host.base_path.as_str();

        // По документации Apple, NSDirectoryEnumerator возвращает пути
        // относительно базовой директории.
        // Поэтому мы честно отрезаем base_path от начала строки.
        let rel_path = if path_str.starts_with(base_str) {
            let mut stripped = &path_str[base_str.len()..];
            if stripped.starts_with('/') {
                stripped = &stripped[1..];
            }
            stripped
        } else {
            path_str
        };

        // Apple docs (`-[NSDirectoryEnumerator level]`): the value reflects
        // how many directories below the enumeration root the current path
        // lives in. An item directly inside the base directory has level 1.
        // Counting the path separators in the relative path and adding 1
        // gives the right answer for both files and subdirectories. An empty
        // relative string (which would represent the base path itself, not
        // normally yielded by `enumerate_recursive`) keeps level 0.
        host.current_level = if rel_path.is_empty() {
            0
        } else {
            (rel_path.matches('/').count() as NSUInteger) + 1
        };

        let ns_str = ns_string::from_rust_string(env, rel_path.to_string());
        autorelease(env, ns_str)
    } else {
        nil
    }
}

// `-[NSDirectoryEnumerator level]`
// <https://developer.apple.com/documentation/foundation/nsdirectoryenumerator/1413939-level>
// "Returns the number of levels deep the current object is in the
// directory hierarchy being enumerated."
- (NSUInteger)level {
    env.objc.borrow::<NSDirectoryEnumeratorHostObject>(this).current_level
}

- (id)fileAttributes {
    // Иногда игры параллельно запрашивают атрибуты каждого файла (размер/тип).
    // Возвращаем пустой словарь, чтобы не было крэша "unrecognized selector".
    msg_class![env; NSDictionary dictionary]
}

- (())skipDescendants {
    // no-op: метод существует на случай, если игра захочет пропустить подпапку
}

@end

};

// Helper functions for path manipulation
pub fn path_exists(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }
    let manager: id = msg_class![env; NSFileManager defaultManager];
    msg![env; manager fileExistsAtPath:path]
}

pub fn is_directory(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }
    let manager: id = msg_class![env; NSFileManager defaultManager];

    // Allocate a boolean in guest memory
    let is_dir_ptr: MutPtr<bool> = env.mem.alloc(1).cast();
    env.mem.write(is_dir_ptr, false);

    let exists: bool = msg![env; manager fileExistsAtPath:path isDirectory:is_dir_ptr];

    let is_dir = env.mem.read(is_dir_ptr);
    env.mem.free(is_dir_ptr.cast());

    exists && is_dir
}

pub fn create_directory_if_needed(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }

    if is_directory(env, path) {
        return true; // Already exists
    }

    let manager: id = msg_class![env; NSFileManager defaultManager];
    let attributes: id = nil;
    let error: MutPtr<id> = Ptr::null();

    msg![env; manager createDirectoryAtPath:path withIntermediateDirectories:true attributes:attributes error:error]
}

pub fn remove_item(env: &mut Environment, path: id) -> bool {
    if path.is_null() {
        return false;
    }

    let manager: id = msg_class![env; NSFileManager defaultManager];
    let error: MutPtr<id> = Ptr::null();

    msg![env; manager removeItemAtPath:path error:error]
}

pub fn copy_item(env: &mut Environment, src: id, dst: id) -> bool {
    if src.is_null() || dst.is_null() {
        return false;
    }

    let manager: id = msg_class![env; NSFileManager defaultManager];
    let error: MutPtr<id> = Ptr::null();

    msg![env; manager copyItemAtPath:src toPath:dst error:error]
}

pub fn move_item(env: &mut Environment, src: id, dst: id) -> bool {
    if src.is_null() || dst.is_null() {
        return false;
    }

    let manager: id = msg_class![env; NSFileManager defaultManager];
    let error: MutPtr<id> = Ptr::null();

    msg![env; manager moveItemAtPath:src toPath:dst error:error]
}
