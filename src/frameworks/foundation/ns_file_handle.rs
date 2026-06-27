/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSFileHandle`, `NSPipe`, and `NSCache`.
//!
//! References:
//!  * Apple documentation: <https://developer.apple.com/documentation/foundation/nsfilehandle>
//!  * Apple documentation: <https://developer.apple.com/documentation/foundation/nspipe>
//!  * Apple documentation: <https://developer.apple.com/documentation/foundation/nscache>
//!
//! Note: `NSTask` is a macOS-only class; it does not exist on iPhone OS / iOS
//! and is intentionally not implemented here.

use super::ns_string;
use super::NSUInteger;
use crate::libc::posix_io;
use crate::mem::{ConstPtr, ConstVoidPtr};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

/// One in-process pipe — shared between the reading and writing
/// `NSFileHandle`s vended by an `NSPipe`.
struct PipeBuffer {
    /// Bytes that have been written but not yet read.
    data: VecDeque<u8>,
    /// Whether the writing end is still open. When the writer closes and the
    /// buffer is empty, subsequent reads return empty data (EOF).
    #[allow(dead_code)]
    write_end_open: bool,
    /// Whether the reading end is still open. When false, subsequent writes
    /// are dropped (writing to a closed pipe normally raises SIGPIPE; we treat
    /// it as a warning instead of crashing the emulator).
    read_end_open: bool,
}

/// Identifies whether an `NSFileHandle` is the read or write end of a pipe.
#[derive(Copy, Clone, PartialEq, Eq)]
enum PipeEndKind {
    Reader,
    Writer,
}

/// One end of an in-process `NSPipe`.
struct PipeEnd {
    buffer: Rc<RefCell<PipeBuffer>>,
    kind: PipeEndKind,
}

#[derive(Default)]
struct NSFileHandleHostObject {
    /// POSIX file descriptor backing the handle, or `-1` if this handle is
    /// backed by an in-process pipe (see `pipe`).
    fd: posix_io::FileDescriptor,
    /// `Some` if this handle is one end of an `NSPipe`.
    pipe: Option<PipeEnd>,
}
impl HostObject for NSFileHandleHostObject {}

impl NSFileHandleHostObject {
    fn new_fd(fd: posix_io::FileDescriptor) -> Self {
        NSFileHandleHostObject { fd, pipe: None }
    }
    fn new_pipe(buffer: Rc<RefCell<PipeBuffer>>, kind: PipeEndKind) -> Self {
        NSFileHandleHostObject {
            fd: -1,
            pipe: Some(PipeEnd { buffer, kind }),
        }
    }
}

struct NSPipeHostObject {
    /// `NSFileHandle*` for the reading end — retained.
    file_handle_for_reading: id,
    /// `NSFileHandle*` for the writing end — retained.
    file_handle_for_writing: id,
}
impl Default for NSPipeHostObject {
    // Phantom-fallback value; `nil` handles match `allocWithZone:` before `-init`.
    fn default() -> Self {
        NSPipeHostObject {
            file_handle_for_reading: nil,
            file_handle_for_writing: nil,
        }
    }
}
impl HostObject for NSPipeHostObject {}

/// Per-entry bookkeeping for `NSCache`.
struct NSCacheEntry {
    /// The cached object (retained by the backing `NSMutableDictionary`).
    key: id,
    /// Cost reported via `setObject:forKey:cost:`. `0` for the costless setter.
    cost: u32,
}

struct NSCacheHostObject {
    /// Backing store — `NSMutableDictionary*` retained.
    dict: id,
    /// `NSString*` — cache name. Retained.
    name: id,
    /// Maximum object count (`0` = unlimited).
    count_limit: NSUInteger,
    /// Maximum total cost (`0` = unlimited).
    total_cost_limit: NSUInteger,
    /// Whether evicted objects are sent `-cache:willEvictObject:`.
    evicts_objects_with_discarded_content: bool,
    /// Delegate — weak.
    delegate: id,
    /// Insertion-order list of keys; used to evict the oldest entry first
    /// when the count or cost limit is exceeded. Apple's `NSCache` doesn't
    /// guarantee a specific eviction policy beyond "unpredictable"; we use
    /// FIFO so the behaviour is at least deterministic.
    order: VecDeque<NSCacheEntry>,
    /// Sum of the `cost` of every entry currently in the cache.
    total_cost: u64,
}
impl Default for NSCacheHostObject {
    // Phantom-fallback value; matches `allocWithZone:` before `-init`.
    fn default() -> Self {
        NSCacheHostObject {
            dict: nil,
            name: nil,
            count_limit: 0,
            total_cost_limit: 0,
            evicts_objects_with_discarded_content: true,
            delegate: nil,
            order: VecDeque::new(),
            total_cost: 0,
        }
    }
}
impl HostObject for NSCacheHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFileHandle: NSObject

+ (id)fileHandleForReadingAtPath:(id)path { // NSString*
    log_dbg!("fileHandleForReadingAtPath {}", ns_string::to_rust_string(env, path));
    let path_str: ConstPtr<u8> = msg![env; path UTF8String];
    match posix_io::open_direct(env, path_str, posix_io::O_RDONLY) {
        -1 => nil,
        fd => {
            let host_object = Box::new(NSFileHandleHostObject::new_fd(fd));
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
            let host_object = Box::new(NSFileHandleHostObject::new_fd(fd));
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
            let host_object = Box::new(NSFileHandleHostObject::new_fd(fd));
            let new = env.objc.alloc_object(this, host_object, &mut env.mem);
            autorelease(env, new)
        },
    }
}

- (i32)fileDescriptor {
    env.objc.borrow::<NSFileHandleHostObject>(this).fd
}

- (i64)offsetInFile {
    let host = env.objc.borrow::<NSFileHandleHostObject>(this);
    if host.pipe.is_some() {
        log!("Warning: NSFileHandle offsetInFile called on a pipe handle; returning 0.");
        return 0;
    }
    let fd = host.fd;
    match posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR) {
        -1 => {
            log!("Warning: NSFileHandle offsetInFile failed for fd {}; returning 0.", fd);
            0
        }
        // TODO: What's the correct behaviour if the position is beyond 2GiB?
        cur_pos => cur_pos,
    }
}

- (())seekToFileOffset:(i64)offset {
    let host = env.objc.borrow::<NSFileHandleHostObject>(this);
    if host.pipe.is_some() {
        log!("Warning: NSFileHandle seekToFileOffset:{} called on a pipe handle; ignoring.", offset);
        return;
    }
    let fd = host.fd;
    if posix_io::lseek(env, fd, offset, posix_io::SEEK_SET) == -1 {
        log!(
            "Warning: NSFileHandle seekToFileOffset:{} failed for fd {}; ignoring.",
            offset, fd
        );
    }
}

- (i64)seekToEndOfFile {
    let host = env.objc.borrow::<NSFileHandleHostObject>(this);
    if host.pipe.is_some() {
        log!("Warning: NSFileHandle seekToEndOfFile called on a pipe handle; returning 0.");
        return 0;
    }
    let fd = host.fd;
    match posix_io::lseek(env, fd, 0, posix_io::SEEK_END) {
        -1 => {
            log!("Warning: NSFileHandle seekToEndOfFile failed for fd {}; returning 0.", fd);
            0
        }
        cur_pos => cur_pos,
    }
}

- (id)readDataOfLength:(NSUInteger)length { // NSData*
    let pipe = {
        let host = env.objc.borrow::<NSFileHandleHostObject>(this);
        host.pipe.as_ref().map(|p| (p.buffer.clone(), p.kind))
    };
    if let Some((buffer, kind)) = pipe {
        if kind == PipeEndKind::Writer {
            log!("Warning: NSFileHandle readDataOfLength: called on write end of a pipe; returning empty NSData.");
            return msg_class![env; NSData data];
        }
        let bytes: Vec<u8> = {
            let mut buf = buffer.borrow_mut();
            let take = std::cmp::min(length as usize, buf.data.len());
            buf.data.drain(..take).collect()
        };
        if bytes.is_empty() {
            return msg_class![env; NSData data];
        }
        let len_u32: NSUInteger = bytes.len().try_into().unwrap_or(NSUInteger::MAX);
        let alloc = env.mem.alloc(len_u32);
        env.mem.bytes_at_mut(alloc.cast(), len_u32).copy_from_slice(&bytes);
        return msg_class![env; NSData dataWithBytesNoCopy:alloc length:len_u32];
    }

    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    let buffer = env.mem.alloc(length);
    match posix_io::read(env, fd, buffer, length) {
        -1 => {
            log!(
                "Warning: NSFileHandle readDataOfLength:{} failed for fd {}; \
                 returning empty NSData.",
                length, fd
            );
            env.mem.free(buffer);
            msg_class![env; NSData data]
        }
        bytes_read => {
            let bytes_read_u32: NSUInteger = bytes_read.try_into().unwrap_or(0);
            if bytes_read_u32 != length {
                log!(
                    "Warning: NSFileHandle readDataOfLength: short read on fd {} \
                     (wanted {}, got {}); returning what was read.",
                    fd, length, bytes_read_u32
                );
            }
            msg_class![env; NSData dataWithBytesNoCopy:buffer length:bytes_read_u32]
        }
    }
}

- (id)readDataToEndOfFile {
    let pipe = {
        let host = env.objc.borrow::<NSFileHandleHostObject>(this);
        host.pipe.as_ref().map(|p| (p.buffer.clone(), p.kind))
    };
    if let Some((buffer, kind)) = pipe {
        if kind == PipeEndKind::Writer {
            log!("Warning: NSFileHandle readDataToEndOfFile called on write end of a pipe; returning empty NSData.");
            return msg_class![env; NSData data];
        }
        let bytes: Vec<u8> = {
            let mut buf = buffer.borrow_mut();
            buf.data.drain(..).collect()
        };
        if bytes.is_empty() {
            return msg_class![env; NSData data];
        }
        let len_u32: NSUInteger = bytes.len().try_into().unwrap_or(NSUInteger::MAX);
        let alloc = env.mem.alloc(len_u32);
        env.mem.bytes_at_mut(alloc.cast(), len_u32).copy_from_slice(&bytes);
        return msg_class![env; NSData dataWithBytesNoCopy:alloc length:len_u32];
    }

    let offset: i64 = msg![env; this offsetInFile];
    let eof: i64 = msg![env; this seekToEndOfFile];
    let _: () = msg![env; this seekToFileOffset:offset];
    let delta = eof.saturating_sub(offset);
    let Ok(length): Result<NSUInteger, _> = delta.try_into() else {
        log!(
            "Warning: NSFileHandle readDataToEndOfFile: negative or oversize range \
             ({} bytes); returning empty NSData.",
            delta
        );
        return msg_class![env; NSData data];
    };

    msg![env; this readDataOfLength:length]
}

- (id)availableData {
    msg![env; this readDataToEndOfFile]
}

- (())writeData:(id)data { // NSData *
    let bytes_ptr: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];

    let pipe = {
        let host = env.objc.borrow::<NSFileHandleHostObject>(this);
        host.pipe.as_ref().map(|p| (p.buffer.clone(), p.kind))
    };
    if let Some((buffer, kind)) = pipe {
        if kind == PipeEndKind::Reader {
            log!("Warning: NSFileHandle writeData: called on read end of a pipe; ignoring.");
            return;
        }
        let slice = env.mem.bytes_at(bytes_ptr.cast(), length);
        let mut buf = buffer.borrow_mut();
        if !buf.read_end_open {
            log!("Warning: NSFileHandle writeData: read end of pipe is closed; ignoring {} bytes.", length);
            return;
        }
        buf.data.extend(slice.iter().copied());
        return;
    }

    let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
    if posix_io::write(env, fd, bytes_ptr, length) == -1 {
        log!("Warning: NSFileHandle writeData: failed for fd {}; ignoring.", fd);
    }
}

- (())synchronizeFile {
    let host = env.objc.borrow::<NSFileHandleHostObject>(this);
    if host.pipe.is_some() {
        // Pipes have no on-disk state to flush.
        return;
    }
    let fd = host.fd;
    if posix_io::fsync(env, fd) == -1 {
        log_dbg!("synchronizeFile: fsync failed for fd {}", fd);
    }
}

- (())truncateFileAtOffset:(i64)offset {
    let host = env.objc.borrow::<NSFileHandleHostObject>(this);
    if host.pipe.is_some() {
        log!("Warning: NSFileHandle truncateFileAtOffset:{} called on a pipe handle; ignoring.", offset);
        return;
    }
    let fd = host.fd;
    if posix_io::ftruncate(env, fd, offset) == -1 {
        log!(
            "Warning: NSFileHandle truncateFileAtOffset:{} failed for fd {}; ignoring.",
            offset, fd
        );
    }
}

- (())closeFile {
    // file is closed on dealloc
    // TODO: keep closed state and raise an exception
    // if handle is used after the closing
    let pipe = {
        let host = env.objc.borrow::<NSFileHandleHostObject>(this);
        host.pipe.as_ref().map(|p| (p.buffer.clone(), p.kind))
    };
    if let Some((buffer, kind)) = pipe {
        let mut buf = buffer.borrow_mut();
        match kind {
            PipeEndKind::Reader => buf.read_end_open = false,
            PipeEndKind::Writer => buf.write_end_open = false,
        }
    }
}

- (())dealloc {
    let pipe = {
        let host = env.objc.borrow::<NSFileHandleHostObject>(this);
        host.pipe.as_ref().map(|p| (p.buffer.clone(), p.kind))
    };
    if let Some((buffer, kind)) = pipe {
        let mut buf = buffer.borrow_mut();
        match kind {
            PipeEndKind::Reader => buf.read_end_open = false,
            PipeEndKind::Writer => buf.write_end_open = false,
        }
    } else {
        let fd = env.objc.borrow::<NSFileHandleHostObject>(this).fd;
        if fd >= 0 {
            posix_io::close(env, fd);
        }
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// =========================================================================
// MARK: - NSPipe
// =========================================================================
//
// Apple documentation: https://developer.apple.com/documentation/foundation/nspipe
//
// NSPipe is an object-oriented wrapper around a one-way IPC channel. Bytes
// written to `fileHandleForWriting` become available on `fileHandleForReading`.
// On Apple's platforms NSPipe wraps a kernel pipe(2); we emulate it with an
// in-process byte buffer so that an app can use the read/write file handles
// to talk to itself (the only thing it can do anyway, since iOS apps cannot
// create subprocesses).

@implementation NSPipe: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSPipeHostObject {
        file_handle_for_reading: nil,
        file_handle_for_writing: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)pipe {
    let inst: id = msg![env; this alloc];
    let inst: id = msg![env; inst init];
    autorelease(env, inst)
}

- (id)init {
    let buffer = Rc::new(RefCell::new(PipeBuffer {
        data: VecDeque::new(),
        write_end_open: true,
        read_end_open: true,
    }));
    let file_handle_class = env.objc.get_known_class("NSFileHandle", &mut env.mem);

    let reading = env.objc.alloc_object(
        file_handle_class,
        Box::new(NSFileHandleHostObject::new_pipe(buffer.clone(), PipeEndKind::Reader)),
        &mut env.mem,
    );

    let writing = env.objc.alloc_object(
        file_handle_class,
        Box::new(NSFileHandleHostObject::new_pipe(buffer, PipeEndKind::Writer)),
        &mut env.mem,
    );

    let h = env.objc.borrow_mut::<NSPipeHostObject>(this);
    h.file_handle_for_reading = reading;
    h.file_handle_for_writing = writing;
    this
}

- (())dealloc {
    let (r, w) = {
        let h = env.objc.borrow::<NSPipeHostObject>(this);
        (h.file_handle_for_reading, h.file_handle_for_writing)
    };
    release(env, r);
    release(env, w);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)fileHandleForReading { // NSFileHandle*
    env.objc.borrow::<NSPipeHostObject>(this).file_handle_for_reading
}

- (id)fileHandleForWriting { // NSFileHandle*
    env.objc.borrow::<NSPipeHostObject>(this).file_handle_for_writing
}

- (id)description {
    let s = format!("<NSPipe: {:?}>", this);
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - NSCache
// =========================================================================
//
// Apple documentation: https://developer.apple.com/documentation/foundation/nscache
//
// Available since iOS 4.0. The cache stores `(key, value)` pairs and may
// evict entries automatically to respect `countLimit` and `totalCostLimit`.
// Apple makes no guarantees about which entries are evicted first; this
// implementation evicts in insertion order (FIFO).

@implementation NSCache: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSCacheHostObject {
        dict: nil,
        name: nil,
        count_limit: 0,
        total_cost_limit: 0,
        evicts_objects_with_discarded_content: true,
        delegate: nil,
        order: VecDeque::new(),
        total_cost: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    let dict: id = msg_class![env; NSMutableDictionary new];
    let empty_name = ns_string::get_static_str(env, "");
    retain(env, empty_name);
    let h = env.objc.borrow_mut::<NSCacheHostObject>(this);
    h.dict = dict;
    h.name = empty_name;
    this
}

- (())dealloc {
    let (dict, name) = {
        let h = env.objc.borrow::<NSCacheHostObject>(this);
        (h.dict, h.name)
    };
    release(env, dict);
    release(env, name);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Name

- (id)name { // NSString*
    env.objc.borrow::<NSCacheHostObject>(this).name
}
- (())setName:(id)name {
    // Standard "retain new, release old" pattern — safe when new == old.
    retain(env, name);
    let old = env.objc.borrow::<NSCacheHostObject>(this).name;
    env.objc.borrow_mut::<NSCacheHostObject>(this).name = name;
    release(env, old);
}

// MARK: Limits

- (NSUInteger)countLimit {
    env.objc.borrow::<NSCacheHostObject>(this).count_limit
}
- (())setCountLimit:(NSUInteger)limit {
    env.objc.borrow_mut::<NSCacheHostObject>(this).count_limit = limit;
    enforce_limits(env, this);
}

- (NSUInteger)totalCostLimit {
    env.objc.borrow::<NSCacheHostObject>(this).total_cost_limit
}
- (())setTotalCostLimit:(NSUInteger)limit {
    env.objc.borrow_mut::<NSCacheHostObject>(this).total_cost_limit = limit;
    enforce_limits(env, this);
}

- (bool)evictsObjectsWithDiscardedContent {
    env.objc.borrow::<NSCacheHostObject>(this).evicts_objects_with_discarded_content
}
- (())setEvictsObjectsWithDiscardedContent:(bool)v {
    env.objc.borrow_mut::<NSCacheHostObject>(this).evicts_objects_with_discarded_content = v;
}

// MARK: Delegate

- (id)delegate {
    env.objc.borrow::<NSCacheHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    // Weak — no retain (Apple's NSCache holds a non-owning reference).
    env.objc.borrow_mut::<NSCacheHostObject>(this).delegate = delegate;
}

// MARK: Access

- (id)objectForKey:(id)key {
    let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
    if dict == nil || key == nil { return nil; }
    msg![env; dict objectForKey:key]
}

- (())setObject:(id)obj forKey:(id)key {
    let _: () = msg![env; this setObject:obj forKey:key cost:0u32];
}

- (())setObject:(id)obj forKey:(id)key cost:(u32)cost {
    if key == nil || obj == nil { return; }
    let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
    if dict == nil { return; }

    // If the key already exists, subtract the old cost from the running total
    // and remove the existing order entry before re-inserting.
    let existing: id = msg![env; dict objectForKey:key];
    if existing != nil {
        let h = env.objc.borrow_mut::<NSCacheHostObject>(this);
        if let Some(pos) = h.order.iter().position(|e| e.key == key) {
            let old_cost = h.order[pos].cost as u64;
            h.total_cost = h.total_cost.saturating_sub(old_cost);
            h.order.remove(pos);
        }
    }

    // Record the new (key, cost) pair in insertion order. The key is retained
    // by the backing dictionary, so we don't retain it again here.
    {
        let h = env.objc.borrow_mut::<NSCacheHostObject>(this);
        h.order.push_back(NSCacheEntry { key, cost });
        h.total_cost = h.total_cost.saturating_add(cost as u64);
    }

    let _: () = msg![env; dict setObject:obj forKey:key];

    enforce_limits(env, this);
}

- (())removeObjectForKey:(id)key {
    let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
    if dict == nil || key == nil { return; }
    {
        let h = env.objc.borrow_mut::<NSCacheHostObject>(this);
        if let Some(pos) = h.order.iter().position(|e| e.key == key) {
            let entry_cost = h.order[pos].cost as u64;
            h.total_cost = h.total_cost.saturating_sub(entry_cost);
            h.order.remove(pos);
        }
    }
    let _: () = msg![env; dict removeObjectForKey:key];
}

- (())removeAllObjects {
    let dict = env.objc.borrow::<NSCacheHostObject>(this).dict;
    if dict == nil { return; }
    {
        let h = env.objc.borrow_mut::<NSCacheHostObject>(this);
        h.order.clear();
        h.total_cost = 0;
    }
    let _: () = msg![env; dict removeAllObjects];
}

// MARK: Description

- (id)description {
    let (name, count) = {
        let h = env.objc.borrow::<NSCacheHostObject>(this);
        let name = h.name;
        let dict = h.dict;
        let count: NSUInteger = if dict != nil { msg![env; dict count] } else { 0 };
        let name_str = if name != nil {
            ns_string::to_rust_string(env, name).into_owned()
        } else {
            String::new()
        };
        (name_str, count)
    };
    let s = format!("<NSCache: {:?}; name={:?}; count={}>", this, name, count);
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

/// Drop entries from the cache (in insertion order) until the configured
/// `countLimit` and `totalCostLimit` are satisfied. A limit of `0` means
/// "no limit" per Apple's documentation.
fn enforce_limits(env: &mut crate::Environment, cache: id) {
    loop {
        let (over_count, over_cost, victim_key, victim_cost) = {
            let h = env.objc.borrow::<NSCacheHostObject>(cache);
            let count = h.order.len() as NSUInteger;
            let over_count = h.count_limit > 0 && count > h.count_limit;
            let over_cost = h.total_cost_limit > 0
                && h.total_cost > h.total_cost_limit as u64;
            let victim = h.order.front();
            (
                over_count,
                over_cost,
                victim.map(|e| e.key).unwrap_or(nil),
                victim.map(|e| e.cost).unwrap_or(0),
            )
        };
        if !over_count && !over_cost { break; }
        if victim_key == nil { break; }

        let (dict, delegate) = {
            let h = env.objc.borrow::<NSCacheHostObject>(cache);
            (h.dict, h.delegate)
        };

        // -cache:willEvictObject: notification — Apple docs:
        // https://developer.apple.com/documentation/foundation/nscachedelegate/cache(_:willevictobject:)
        if delegate != nil {
            let victim_obj: id = msg![env; dict objectForKey:victim_key];
            if victim_obj != nil {
                let sel = env
                    .objc
                    .lookup_selector("cache:willEvictObject:")
                    .unwrap_or_else(|| {
                        env.objc.register_host_selector(
                            "cache:willEvictObject:".to_string(),
                            &mut env.mem,
                        )
                    });
                let responds: bool = msg![env; delegate respondsToSelector:sel];
                if responds {
                    let _: () = msg![env; delegate cache:cache willEvictObject:victim_obj];
                }
            }
        }

        {
            let h = env.objc.borrow_mut::<NSCacheHostObject>(cache);
            h.order.pop_front();
            h.total_cost = h.total_cost.saturating_sub(victim_cost as u64);
        }
        let _: () = msg![env; dict removeObjectForKey:victim_key];
    }
}
