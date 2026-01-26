/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
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
};
use crate::Environment;

type NSSearchPathDirectory = NSUInteger;
const NSApplicationDirectory: NSSearchPathDirectory = 1;
const NSLibraryDirectory: NSSearchPathDirectory = 5;
const NSDocumentDirectory: NSSearchPathDirectory = 9;

type NSSearchPathDomainMask = NSUInteger;
const NSUserDomainMask: NSSearchPathDomainMask = 1;

pub const NSFileModificationDate: &str = "NSFileModificationDate";
pub const NSFileSize: &str = "NSFileSize";
const NSFileSystemFreeSize: &str = "NSFileSystemFreeSize";

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSFileModificationDate",
        HostConstant::NSString(NSFileModificationDate),
    ),
    ("_NSFileSize", HostConstant::NSString(NSFileSize)),
    (
        "_NSFileSystemFreeSize",
        HostConstant::NSString(NSFileSystemFreeSize),
    ),
];

fn NSSearchPathForDirectoriesInDomains(
    env: &mut Environment,
    directory: NSSearchPathDirectory,
    domain_mask: NSSearchPathDomainMask,
    expand_tilde: bool,
) -> id {
    // TODO: other cases not implemented
    assert!(domain_mask == NSUserDomainMask);
    assert!(expand_tilde);

    let dir = match directory {
        NSApplicationDirectory => {
            // This might not actually be correct. I haven't bothered to
            // test it because I can't think of a good reason an iPhone OS app
            // would have to request this;
            // Wolfenstein 3D requests it but never uses it.
            GuestPath::new(crate::fs::APPLICATIONS).to_owned()
        }
        NSDocumentDirectory => env.fs.home_directory().join("Documents"),
        NSLibraryDirectory => env.fs.home_directory().join("Library"),
        _ => todo!("NSSearchPathDirectory {}", directory),
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

/// Check [crate::fs::Fs::new] for more info for
/// how temporary folder is setup on startup
fn NSTemporaryDirectory(env: &mut Environment) -> id {
    let dir = env.fs.home_directory().join("tmp");
    let dir = ns_string::from_rust_string(env, String::from(dir.as_str()));
    autorelease(env, dir)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NSHomeDirectory()),
    export_c_func!(NSTemporaryDirectory()),
    export_c_func!(NSSearchPathForDirectoriesInDomains(_, _, _)),
];

#[derive(Default)]
pub struct State {
    default_manager: Option<id>,
}

struct NSDirectoryEnumeratorHostObject {
    iterator: std::vec::IntoIter<GuestPathBuf>,
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

- (id)currentDirectoryPath {
    ns_string::from_rust_string(env, env.fs.working_directory().as_str().to_string())
}

- (bool)changeCurrentDirectoryPath:(id)path {
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    let path = GuestPath::new(&path);
    match env.fs.change_working_directory(path) {
        Ok(_) => true,
        Err(()) => false
    }
}

- (bool)fileExistsAtPath:(id)path { // NSString*
    let res_exists = if path == nil {
        false
    } else {
        let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
        // fileExistsAtPath: will return true for directories
        // hence Fs::exists() rather than Fs::is_file() is appropriate.
        env.fs.exists(GuestPath::new(&path))
    };
    log_dbg!("[(NSFileManager*) {:?} fileExistsAtPath:{:?}] => {}", this, path, res_exists);
    res_exists
}

- (bool)fileExistsAtPath:(id)path // NSString*
             isDirectory:(MutPtr<bool>)is_dir {
    let (res_exists, res_is_dir) = if path == nil {
        (false, false)
    } else {
        // TODO: mutualize with fileExistsAtPath:
        let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
        let guest_path = GuestPath::new(&path);
        (env.fs.exists(guest_path), !env.fs.is_file(guest_path))
    };

    if !is_dir.is_null() {
        env.mem.write(is_dir, res_is_dir);
    }

    log_dbg!("[(NSFileManager*) {:?} fileExistsAtPath:{:?} isDirectory:{:?}] => {}", this, path, res_is_dir, res_exists);
    res_exists
}

- (bool)createFileAtPath:(id)path // NSString*
                contents:(id)data // NSData*
              attributes:(id)attributes { // NSDictionary*
    assert!(attributes == nil); // TODO
    if data == nil {
        let empty: id = msg_class![env; NSData new];
        let res: bool = msg![env; empty writeToFile:path atomically:false];
        release(env, empty);
        res
    } else {
        msg![env; data writeToFile:path atomically:false]
    }
}

- (bool)removeItemAtPath:(id)path // NSString*
                   error:(MutPtr<id>)out_error { // NSError**
    // TODO: call delegate
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    match env.fs.remove(GuestPath::new(&path)) {
        Ok(()) => true,
        Err(err) => {
            if !out_error.is_null() {
                match err {
                    FsError::DoesNotExist => {
                        let domain = get_static_str(env, NSCocoaErrorDomain);
                        let error = msg_class![env; NSError alloc];
                        let error = msg![env; error initWithDomain:domain code:NSFileReadNoSuchFileError userInfo:nil];
                        env.mem.write(out_error, error);
                    }
                    _ => unimplemented!()
                }
            }
            false
        }
    }
}

- (bool)moveItemAtPath:(id)path // NSString*
                toPath:(id)toPath // NSString*
                 error:(MutPtr<id>)error { // NSError**
    // TODO: call delegate
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    let toPath = ns_string::to_rust_string(env, toPath); // TODO: avoid copy
    match env.fs.rename(GuestPath::new(&path), GuestPath::new(&toPath)) {
        Ok(()) => true,
        Err(()) => {
            if !error.is_null() {
               todo!(); // TODO: create an NSError if requested
            }
            false
        }
    }
}

- (bool)createDirectoryAtPath:(id)path // NSString *
                   attributes:(id)attributes { // NSDictionary*
    let error: MutPtr<id> = Ptr::null();
    msg![env; this createDirectoryAtPath:path
             withIntermediateDirectories:false
                              attributes:attributes
                                   error:error]
}

- (bool)createDirectoryAtPath:(id)path // NSString *
  withIntermediateDirectories:(bool)with_intermediates
                   attributes:(id)attributes // NSDictionary*
                        error:(MutPtr<id>)error { // NSError**
    assert_eq!(attributes, nil); // TODO

    let path_str = ns_string::to_rust_string(env, path); // TODO: avoid copy
    let res = if with_intermediates {
        env.fs.create_dir_all(GuestPath::new(&path_str))
    } else {
        env.fs.create_dir(GuestPath::new(&path_str))
    };
    match res {
        Ok(()) => {
            log_dbg!("createDirectoryAtPath {} => true", path_str);
            true
        }
        Err(err) => {
            assert!(error.is_null()); // TODO
            log!(
                "Warning: createDirectoryAtPath {} failed with {:?}, returning false",
                path_str,
                err,
            );
            false
        }
    }
}

- (id)enumeratorAtPath:(id)path { // NSString*
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    let Ok(paths) = env.fs.enumerate_recursive(GuestPath::new(&path)) else {
        return nil;
    };
    let host_object = Box::new(NSDirectoryEnumeratorHostObject {
        iterator: paths.into_iter(),
    });
    let class = env.objc.get_known_class("NSDirectoryEnumerator", &mut env.mem);
    let enumerator = env.objc.alloc_object(class, host_object, &mut env.mem);
    autorelease(env, enumerator)
}

- (id)directoryContentsAtPath:(id)path /* NSString* */ { // NSArray*
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
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

- (id)contentsOfDirectoryAtPath:(id)path /* NSString* */
                          error:(MutPtr<id>)error { // NSError**
    let contents: id = msg![env; this directoryContentsAtPath:path];
    if contents == nil && !error.is_null() {
        todo!(); // TODO: create an NSError if requested
    }
    contents
}

- (bool)isReadableFileAtPath:(id)path { // NSString*
    let (_, readable, _, _) = {
        let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
        env.fs.access(GuestPath::new(&path))
    };
    readable
}

- (bool)isWritableFileAtPath:(id)path { // NSString*
    let (_, _, writable, _) = {
        let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
        env.fs.access(GuestPath::new(&path))
    };
    writable
}

- (bool)isDeletableFileAtPath:(id)path { // NSString*
    let is_file = {
        let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
        env.fs.is_file(GuestPath::new(&path))
    };

    if is_file {
        return msg![env; this isWritableFileAtPath:path];
    }

    let directory_enumerator: id = msg![env; this enumeratorAtPath:path];

    let mut is_deletable = true;
    loop {
        let path: id = msg![env; directory_enumerator nextObject];
        if path == nil {
            break;
        }
        let is_path_deletable: bool = msg![env; this isDeletableFileAtPath:path];
        is_deletable &= is_path_deletable;
        if !is_deletable {
            break;
        }
    }
    is_deletable
}

- (id)contentsAtPath:(id)path { // NSString *
    // TODO: return nil if path is directory
    // TODO: handle non-absolute paths?
    assert!(msg![env; path isAbsolutePath]);
    msg_class![env; NSData dataWithContentsOfFile:path]
}

- (bool)copyItemAtPath:(id)src // NSString*
                toPath:(id)dst // NSString*
                 error:(MutPtr<id>)error { // NSError**
    let src = ns_string::to_rust_string(env, src);
    let dst = ns_string::to_rust_string(env, dst);
    let data = match env.fs.read(GuestPath::new(src.as_ref())) {
        Ok(d) => d,
        Err(_) => {
            assert!(error.is_null()); // TODO
            return false;
        }
    };
    if env.fs.write(GuestPath::new(dst.as_ref()), &data).is_err() {
        assert!(error.is_null()); // TODO
        return false;
    }
    true
}

- (ConstPtr<u8>)fileSystemRepresentationWithPath:(id)path { // NSString*
    let length: NSUInteger = msg![env; path length];
    assert!(length > 0);
    // TODO: throw an exception if conversion fails
    msg![env; path UTF8String]
}

- (id)fileAttributesAtPath:(id)path // NSString *
              traverseLink:(bool)traverse {
    // TODO: other attributes
    log_once!("Warning: NSFileManager fileAttributesAtPath:traverseLink: returns only NSFileModificationDate and NSFileSize attributes!");

    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    // TODO: traverse link
    log_dbg!("[(NSFileManager *){:?} fileAttributesAtPath:{} traverse:{}]", this, path, traverse);
    let guest_path = GuestPath::new(&path);

    file_attributes_common(env, guest_path)
}

- (id)attributesOfItemAtPath:(id)path // NSString *
                       error:(MutPtr<id>)error { // NSError **
    assert!(error.is_null()); // TODO

    // TODO: other attributes
    log_once!("Warning: NSFileManager attributesOfItemAtPath:error: returns only NSFileModificationDate and NSFileSize attributes!");

    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    // TODO: traverse link
    log_dbg!("[(NSFileManager *){:?} attributesOfItemAtPath:{} error:{:?}]", this, path, error);
    let guest_path = GuestPath::new(&path);

    file_attributes_common(env, guest_path)
}

- (id)attributesOfFileSystemForPath:(id)_path
                              error:(MutPtr<id>)error {
    // TODO: other attributes
    log_once!("Warning: NSFileManager attributesOfFileSystemForPath:error: returns only NSFileSystemFreeSize attribute!");

    assert!(error.is_null()); // TODO

    let dict = msg_class![env; NSMutableDictionary new];

    // Reporting 1 Gb of free space should be enough
    // TODO: unify with `statfs`
    // TODO: account for path
    let size: u64 = 1024 * 1024 * 1024;
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let fs_free_size_key = get_static_str(env, NSFileSystemFreeSize);
    () = msg![env; dict setObject:size_num forKey:fs_free_size_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}

@end

@implementation NSDirectoryEnumerator: NSEnumerator

- (id)nextObject {
    let host_obj = env.objc.borrow_mut::<NSDirectoryEnumeratorHostObject>(this);
    host_obj.iterator.next().map_or(nil, |s| ns_string::from_rust_string(env, String::from(s)))
}

@end

};

/// Helper function for `fileAttributesAtPath:traverseLink:` and
/// `attributesOfItemAtPath:error:`
fn file_attributes_common(env: &mut Environment, guest_path: &GuestPath) -> id {
    if !env.fs.exists(guest_path) {
        log!(
            "file_attributes_common() called with file that does not exist: {:?}, Returning nil",
            guest_path
        );
        return nil;
    }

    // TODO: support more attributes
    let unix_timestamp: f64 = env.fs.modified(guest_path).unwrap() as f64;
    let unix_ref_date: id = msg_class![env; NSDate dateWithTimeIntervalSince1970:0f64];
    let unix_date: id =
        msg_class![env; NSDate dateWithTimeInterval:unix_timestamp sinceDate:unix_ref_date];

    let size = env.fs.size(guest_path).unwrap();
    let size_num: id = msg_class![env; NSNumber numberWithUnsignedLongLong:size];

    let dict = msg_class![env; NSMutableDictionary new];

    let modif_date_key = get_static_str(env, NSFileModificationDate);
    () = msg![env; dict setObject:unix_date forKey:modif_date_key];

    let size_key = get_static_str(env, NSFileSize);
    () = msg![env; dict setObject:size_num forKey:size_key];

    let dict_imm = msg![env; dict copy];
    release(env, dict);
    autorelease(env, dict_imm)
}
