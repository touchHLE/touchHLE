/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSFileHandle` etc.

use crate::frameworks::foundation::{ns_data, ns_string, NSUInteger};
use crate::fs::{GuestFile, GuestOpenOptions, GuestPath};
use crate::objc::{autorelease, id, msg_class, nil, HostObject};
use crate::{objc::ClassExports, objc_classes};
use std::io::{Read, Seek, Write};

struct NSFileHandleHostObject {
    file: GuestFile,
}
impl HostObject for NSFileHandleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileHandle: NSObject

+ (id)fileHandleForReadingAtPath:(id)path { //NSString*
    let path_str = ns_string::to_rust_string(env, path);
    let mut options = GuestOpenOptions::new();
    options.read();
    let res = if let Ok(file) = env.fs.open_with_options(GuestPath::new(&path_str), options) {
        let host_object = NSFileHandleHostObject { file };
        let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
        autorelease(env, new)
    } else {
        nil
    };
    log_dbg!("[NSFileHandle fileHandleForReadingAtPath:{:?}] => {:?}", path_str, res);
    res
}

+ (id)fileHandleForWritingAtPath:(id)path { //NSString*
    let path_str = ns_string::to_rust_string(env, path);
    let mut options = GuestOpenOptions::new();
    options.write();
    let res = if let Ok(file) = env.fs.open_with_options(GuestPath::new(&path_str), options) {
        let host_object = NSFileHandleHostObject { file };
        let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
        autorelease(env, new)
    } else {
        nil
    };
    log_dbg!("[NSFileHandle fileHandleForWritingAtPath:{:?}] => {:?}", path_str, res);
    res
}

- (id)readDataOfLength:(NSUInteger)length {
    let res = if length == 0 {
        // Empty NSData
        let new =  msg_class![env; NSData alloc];
        autorelease(env, new)
    } else {
        let mut buffer = vec![0; length as usize];
        let host_object = env.objc.borrow_mut::<NSFileHandleHostObject>(this);
        let read_length = host_object.file.read(&mut buffer[..]).unwrap();
        ns_data::from_rust_slice(env, &buffer[0..read_length])
    };
    log_dbg!("[NSFileHandle* {:?} readDataOfLength:{:?}] => {:?}", this, length, res);
    res
}

- (())writeData:(id)data { //NSData*
    log_dbg!("[NSFileHandle* {:?} writeData:{:?}]", this, data);
    let data = ns_data::to_rust_slice(env, data);
    let mut buffer = vec![0; data.len()];
    buffer[..].clone_from_slice(data);
    let host_object = env.objc.borrow_mut::<NSFileHandleHostObject>(this);
    host_object.file.write_all(&buffer[..]).unwrap();
}

- (u64)offsetInFile {
    let host_object = env.objc.borrow_mut::<NSFileHandleHostObject>(this);
    host_object.file.stream_position().unwrap()
}
- (())seekToFileOffset:(u64)offset {
    let host_object = env.objc.borrow_mut::<NSFileHandleHostObject>(this);
    host_object.file.seek(std::io::SeekFrom::Start(offset)).unwrap();
}
-(())seekToEndOfFile {
    let host_object = env.objc.borrow_mut::<NSFileHandleHostObject>(this);
    host_object.file.seek(std::io::SeekFrom::End(0)).unwrap();
}

- (())closeFile {}

@end

};
