/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSFileHandle`.

use super::ns_string;
use super::NSUInteger;
use crate::libc::posix_io;
use crate::mem::{ConstPtr, ConstVoidPtr};
use crate::objc::{autorelease, id, nil, objc_classes, ClassExports, HostObject};
use crate::{msg, msg_class};

struct NSFileHandleHostObject {
    fd: posix_io::FileDescriptor,
}
impl HostObject for NSFileHandleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileHandle: NSObject

+ (id)fileHandleForReadingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForReadingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_RDONLY) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject {
                fd
            });
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

+ (id)fileHandleForWritingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForWritingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_WRONLY) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject {
                fd
            });
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

+ (id)fileHandleForUpdatingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForUpdatingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_RDWR) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject {
                fd
            });
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

- (i64)offsetInFile {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    match posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR) {
        -1 => panic!("offsetInFile failed"),
        // TODO: What's the correct behaviour if the position is beyond 2GiB?
        cur_pos => cur_pos,
    }
}

- (())seekToFileOffset:(i64)offset {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    match posix_io::lseek(env, fd, offset, posix_io::SEEK_SET) {
        -1 => panic!("seekToFileOffset: failed"),
        _cur_pos => (),
    }
}

- (i64)seekToEndOfFile {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    match posix_io::lseek(env, fd, 0, posix_io::SEEK_END) {
        -1 => panic!("seekToFileOffset: failed"),
        cur_pos => cur_pos,
    }
}

- (id)readDataOfLength:(NSUInteger)length { // NSData*
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    let buffer = env.mem.alloc(length);
    match posix_io::read(env, fd, buffer, length) {
        -1 => panic!("readDataOfLength: failed"),
        bytes_read => {
            assert_eq!(length, bytes_read.try_into().unwrap());
            msg_class![env; NSData dataWithBytesNoCopy:buffer length:length]
        }
    }
}

- (())writeData:(id)data { // NSData *
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];
    if posix_io::write(env, fd, bytes, length) == -1 {
        panic!("writeData: failed")
    }
}

- (())closeFile {
    // file is closed on dealloc
    // TODO: keep closed state and raise an exception
    // if handle is used after the closing
}

- (())dealloc {
    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    posix_io::close(env, fd);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
