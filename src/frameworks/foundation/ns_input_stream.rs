/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSInputStream` and `NSOutputStream`.

use crate::frameworks::foundation::{ns_string, NSInteger, NSUInteger};
use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// MARK: - Stream status / error constants

type NSStreamStatus = NSUInteger;
const NSStreamStatusNotOpen: NSStreamStatus = 0;
const NSStreamStatusOpening: NSStreamStatus = 1;
const NSStreamStatusOpen: NSStreamStatus = 2;
const NSStreamStatusReading: NSStreamStatus = 3;
const NSStreamStatusWriting: NSStreamStatus = 4;
const NSStreamStatusAtEnd: NSStreamStatus = 5;
const NSStreamStatusClosed: NSStreamStatus = 6;
const NSStreamStatusError: NSStreamStatus = 7;

type NSStreamEvent = NSUInteger;
const NSStreamEventNone: NSStreamEvent = 0;
const NSStreamEventOpenCompleted: NSStreamEvent = 1 << 0;
const NSStreamEventHasBytesAvailable: NSStreamEvent = 1 << 1;
const NSStreamEventCanAcceptBytes: NSStreamEvent = 1 << 2;
const NSStreamEventErrorOccurred: NSStreamEvent = 1 << 3;
const NSStreamEventEndEncountered: NSStreamEvent = 1 << 4;

// MARK: - Property key constants (as &str for get_static_str)
pub const NSStreamDataWrittenToMemoryStreamKey: &str = "NSStreamDataWrittenToMemoryStreamKey";
pub const NSStreamFileCurrentOffsetKey: &str = "NSStreamFileCurrentOffsetKey";
pub const NSStreamNetworkServiceType: &str = "NSStreamNetworkServiceType";
pub const NSStreamSOCKSProxyConfigurationKey: &str = "NSStreamSOCKSProxyConfigurationKey";
pub const NSStreamSSLSettings: &str = "NSStreamSSLSettings";

// MARK: - Host objects

#[derive(Default)]
enum InputStreamBacking {
    /// Backed by an in-memory `NSData*`
    Data { data: id, offset: usize },
    /// Backed by a file path — we load it lazily on open.
    File {
        path: id,
        bytes: Vec<u8>,
        offset: usize,
    },
    /// No backing (e.g. network stream stub).
    #[default]
    None,
}

#[derive(Default)]
struct NSInputStreamHostObject {
    backing: InputStreamBacking,
    status: NSStreamStatus,
    /// `id<NSStreamDelegate>` — weak
    delegate: id,
}
impl HostObject for NSInputStreamHostObject {}

#[derive(Default)]
enum OutputStreamBacking {
    /// Buffer grows into a `Vec<u8>`.
    Memory { buffer: Vec<u8> },
    /// File output — stubbed, writes discarded.
    File { path: id },
    /// No backing.
    #[default]
    None,
}

#[derive(Default)]
struct NSOutputStreamHostObject {
    backing: OutputStreamBacking,
    status: NSStreamStatus,
    /// `id<NSStreamDelegate>` — weak
    delegate: id,
}
impl HostObject for NSOutputStreamHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// NSStream (abstract base — concrete methods on subclasses)
// =========================================================================

@implementation NSStream: NSObject

- (id)delegate {
    nil // overridden by subclasses
}
- (())setDelegate:(id)_delegate {}
- (NSStreamStatus)streamStatus { NSStreamStatusNotOpen }
- (id)streamError { nil }
- (())open  {}
- (())close {}
- (id)propertyForKey:(id)_key   { nil }
- (bool)setProperty:(id)_value forKey:(id)_key { false }
- (())scheduleInRunLoop:(id)_run_loop forMode:(id)_mode {}
- (())removeFromRunLoop:(id)_run_loop forMode:(id)_mode {}

@end

// =========================================================================
// NSInputStream
// =========================================================================

@implementation NSInputStream: NSStream

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSInputStreamHostObject {
        backing: InputStreamBacking::None,
        status:  NSStreamStatusNotOpen,
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: Constructors

+ (id)inputStreamWithData:(id)data { // NSData*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithData:data];
    crate::objc::autorelease(env, new)
}

+ (id)inputStreamWithFileAtPath:(id)path { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithFileAtPath:path];
    crate::objc::autorelease(env, new)
}

+ (id)inputStreamWithURL:(id)url { // NSURL*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithURL:url];
    crate::objc::autorelease(env, new)
}

- (id)initWithData:(id)data { // NSData*
    retain(env, data);
    env.objc.borrow_mut::<NSInputStreamHostObject>(this).backing =
        InputStreamBacking::Data { data, offset: 0 };
    this
}

- (id)initWithFileAtPath:(id)path { // NSString*
    retain(env, path);
    env.objc.borrow_mut::<NSInputStreamHostObject>(this).backing =
        InputStreamBacking::File { path, bytes: Vec::new(), offset: 0 };
    this
}

- (id)initWithURL:(id)url { // NSURL*
    let path: id = msg![env; url path];
    msg![env; this initWithFileAtPath:path]
}

// MARK: Dealloc

- (())dealloc {
    let delegate = env.objc.borrow::<NSInputStreamHostObject>(this).delegate;
    // delegate is weak — no release
    let _ = delegate;
    // Release backing data/path.
    match &env.objc.borrow::<NSInputStreamHostObject>(this).backing {
        InputStreamBacking::Data { data, .. } => {
            let d = *data;
            release(env, d);
        }
        InputStreamBacking::File { path, .. } => {
            let p = *path;
            release(env, p);
        }
        InputStreamBacking::None => {}
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)delegate {
    env.objc.borrow::<NSInputStreamHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    // Weak — no retain.
    env.objc.borrow_mut::<NSInputStreamHostObject>(this).delegate = delegate;
}

// MARK: Open / Close

- (())open {
    // 1. Извлекаем нужные данные без долгого заимствования
    let path_to_load = {
        let host = env.objc.borrow_mut::<NSInputStreamHostObject>(this);
        host.status = NSStreamStatusOpen;
        if let InputStreamBacking::File { path, .. } = &host.backing {
            Some(*path)
        } else {
            None
        }
    };

    // 2. Делаем операции, требующие полного env, когда host уже отпущен
    if let Some(path) = path_to_load {
        let path_str = ns_string::to_rust_string(env, path).into_owned();
        let read_res = env.fs.read(crate::fs::GuestPath::new(&path_str));

        // 3. Снова заимствуем host, чтобы записать результат
        let host = env.objc.borrow_mut::<NSInputStreamHostObject>(this);
        if let InputStreamBacking::File { bytes, .. } = &mut host.backing {
            match read_res {
                Ok(data) => *bytes = data,
                Err(_) => {
                    log!("NSInputStream open: couldn't read {:?}", path_str);
                    host.status = NSStreamStatusError;
                }
            }
        }
    }
}

- (())close {
    env.objc.borrow_mut::<NSInputStreamHostObject>(this).status = NSStreamStatusClosed;
}

// MARK: Status / error

- (NSStreamStatus)streamStatus {
    env.objc.borrow::<NSInputStreamHostObject>(this).status
}

- (id)streamError {
    nil // no real error object
}

// MARK: Reading

- (NSInteger)read:(MutPtr<u8>)buffer maxLength:(NSUInteger)len {
    let status = env.objc.borrow::<NSInputStreamHostObject>(this).status;
    if status == NSStreamStatusClosed || status == NSStreamStatusError {
        return -1;
    }

    // Извлекаем тип backing и текущие параметры без долгого заимствования
    let backing_kind = {
        let host = env.objc.borrow::<NSInputStreamHostObject>(this);
        match &host.backing {
            InputStreamBacking::Data { data, offset } => Some((true, *data, *offset)),
            InputStreamBacking::File { offset, .. } => Some((false, nil, *offset)),
            InputStreamBacking::None => None,
        }
    };

    let Some((is_data, data_val, current_offset)) = backing_kind else {
        return -1;
    };

    if is_data {
        // Мы отпустили borrow, теперь смело используем msg![env; ...]
        let bytes_ptr: crate::mem::ConstVoidPtr = msg![env; data_val bytes];
        let total: NSUInteger = msg![env; data_val length];
        let remaining = total as usize - current_offset;
        let to_read = remaining.min(len as usize);

        if to_read == 0 {
            env.objc.borrow_mut::<NSInputStreamHostObject>(this).status = NSStreamStatusAtEnd;
            return 0;
        }

        // Защита от E0502: копируем во временный вектор, чтобы не держать
        // одновременные borrow
        let src_slice = env.mem.bytes_at(bytes_ptr.cast::<u8>() + current_offset as u32, to_read as u32).to_vec();
        env.mem.bytes_at_mut(buffer, to_read as u32).copy_from_slice(&src_slice);

        // Обновляем состояние
        let host = env.objc.borrow_mut::<NSInputStreamHostObject>(this);
        if let InputStreamBacking::Data { offset, .. } = &mut host.backing {
            *offset += to_read;
            if *offset >= total as usize {
                host.status = NSStreamStatusAtEnd;
            }
        }
        return to_read as NSInteger;
    } else {
        // Это File backing
        let chunk = {
            let host = env.objc.borrow::<NSInputStreamHostObject>(this);
            if let InputStreamBacking::File { bytes, offset, .. } = &host.backing {
                let remaining = bytes.len() - *offset;
                let to_read = remaining.min(len as usize);
                if to_read == 0 {
                    Vec::new()
                } else {
                    bytes[*offset..*offset + to_read].to_vec()
                }
            } else {
                Vec::new()
            }
        };

        if chunk.is_empty() {
            env.objc.borrow_mut::<NSInputStreamHostObject>(this).status = NSStreamStatusAtEnd;
            return 0;
        }

        env.mem.bytes_at_mut(buffer, chunk.len() as u32).copy_from_slice(&chunk);

        let host = env.objc.borrow_mut::<NSInputStreamHostObject>(this);
        if let InputStreamBacking::File { bytes, offset, .. } = &mut host.backing {
            *offset += chunk.len();
            if *offset >= bytes.len() {
                host.status = NSStreamStatusAtEnd;
            }
        }
        return chunk.len() as NSInteger;
    }
}

- (bool)hasBytesAvailable {
    let backing_kind = {
        let host = env.objc.borrow::<NSInputStreamHostObject>(this);
        match &host.backing {
            InputStreamBacking::Data { data, offset } => Some((*data, *offset)),
            _ => None,
        }
    };

    if let Some((data_val, current_offset)) = backing_kind {
        let total: NSUInteger = msg![env; data_val length];
        return (current_offset as NSUInteger) < total;
    }

    match &env.objc.borrow::<NSInputStreamHostObject>(this).backing {
        InputStreamBacking::File { bytes, offset, .. } => *offset < bytes.len(),
        _ => false,
    }
}

- (bool)getBuffer:(MutPtr<MutPtr<u8>>)_buffer length:(MutPtr<NSUInteger>)_len {
    // Optional optimisation — we don't support it.
    false
}

// MARK: Properties

- (id)propertyForKey:(id)key {
    let key_str = ns_string::to_rust_string(env, key);
    match key_str.as_ref() {
        NSStreamFileCurrentOffsetKey => {
            let offset = match &env.objc.borrow::<NSInputStreamHostObject>(this).backing {
                InputStreamBacking::Data { offset, .. } => *offset as i64,
                InputStreamBacking::File { offset, .. } => *offset as i64,
                InputStreamBacking::None => 0,
            };
            msg_class![env; NSNumber numberWithLongLong:offset]
        }
        _ => nil,
    }
}

- (bool)setProperty:(id)value forKey:(id)key {
    let key_str = ns_string::to_rust_string(env, key);
    match key_str.as_ref() {
        NSStreamFileCurrentOffsetKey => {
            let new_offset: i64 = msg![env; value longLongValue];
            match &mut env.objc.borrow_mut::<NSInputStreamHostObject>(this).backing {
                InputStreamBacking::Data { offset, .. } => *offset = new_offset as usize,
                InputStreamBacking::File { offset, .. } => *offset = new_offset as usize,
                InputStreamBacking::None => {}
            }
            true
        }
        _ => false,
    }
}

- (())scheduleInRunLoop:(id)_run_loop forMode:(id)_mode {
    log_dbg!("NSInputStream scheduleInRunLoop:forMode: stubbed");
}

- (())removeFromRunLoop:(id)_run_loop forMode:(id)_mode {
    log_dbg!("NSInputStream removeFromRunLoop:forMode: stubbed");
}

@end

// =========================================================================
// NSOutputStream
// =========================================================================

@implementation NSOutputStream: NSStream

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSOutputStreamHostObject {
        backing: OutputStreamBacking::None,
        status:  NSStreamStatusNotOpen,
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: Constructors

+ (id)outputStreamToMemory {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initToMemory];
    crate::objc::autorelease(env, new)
}

+ (id)outputStreamToBuffer:(MutPtr<u8>)buffer capacity:(NSUInteger)_capacity {
    // We ignore the fixed buffer and use memory backing instead.
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initToMemory];
    crate::objc::autorelease(env, new)
}

+ (id)outputStreamWithURL:(id)url append:(bool)should_append {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithURL:url append:should_append];
    crate::objc::autorelease(env, new)
}

+ (id)outputStreamToFileAtPath:(id)path append:(bool)should_append {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initToFileAtPath:path append:should_append];
    crate::objc::autorelease(env, new)
}

- (id)initToMemory {
    env.objc.borrow_mut::<NSOutputStreamHostObject>(this).backing =
        OutputStreamBacking::Memory { buffer: Vec::new() };
    this
}

- (id)initToBuffer:(MutPtr<u8>)_buffer capacity:(NSUInteger)_capacity {
    // Use memory backing.
    msg![env; this initToMemory]
}

- (id)initToFileAtPath:(id)path append:(bool)_append {
    retain(env, path);
    env.objc.borrow_mut::<NSOutputStreamHostObject>(this).backing =
        OutputStreamBacking::File { path };
    this
}

- (id)initWithURL:(id)url append:(bool)append {
    let path: id = msg![env; url path];
    msg![env; this initToFileAtPath:path append:append]
}

// MARK: Dealloc

- (())dealloc {
    if let OutputStreamBacking::File { path } =
        &env.objc.borrow::<NSOutputStreamHostObject>(this).backing
    {
        let p = *path;
        release(env, p);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)delegate {
    env.objc.borrow::<NSOutputStreamHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<NSOutputStreamHostObject>(this).delegate = delegate;
}

// MARK: Open / Close

- (())open {
    env.objc.borrow_mut::<NSOutputStreamHostObject>(this).status = NSStreamStatusOpen;
}

- (())close {
    env.objc.borrow_mut::<NSOutputStreamHostObject>(this).status = NSStreamStatusClosed;
}

// MARK: Status

- (NSStreamStatus)streamStatus {
    env.objc.borrow::<NSOutputStreamHostObject>(this).status
}

- (id)streamError {
    nil
}

// MARK: Writing

- (NSInteger)write:(crate::mem::ConstPtr<u8>)buffer maxLength:(NSUInteger)len {
    let status = env.objc.borrow::<NSOutputStreamHostObject>(this).status;
    if status == NSStreamStatusClosed || status == NSStreamStatusError {
        return -1;
    }
    if len == 0 { return 0;
    }

    match &mut env.objc.borrow_mut::<NSOutputStreamHostObject>(this).backing {
        OutputStreamBacking::Memory { buffer: buf } => {
            let bytes = env.mem.bytes_at(buffer, len).to_vec();
            buf.extend_from_slice(&bytes);
            len as NSInteger
        }
        OutputStreamBacking::File { .. } => {
            log!("NSOutputStream write: file output stubbed — bytes discarded");
            len as NSInteger
        }
        OutputStreamBacking::None => -1,
    }
}

- (bool)hasSpaceAvailable {
    let status = env.objc.borrow::<NSOutputStreamHostObject>(this).status;
    status == NSStreamStatusOpen
}

// MARK: Properties

- (id)propertyForKey:(id)key {
    let key_str = ns_string::to_rust_string(env, key);
    match key_str.as_ref() {
        NSStreamDataWrittenToMemoryStreamKey => {
            // Извлекаем клон буфера, не задерживая borrow
            let buffer_clone = if let OutputStreamBacking::Memory { buffer } =
                &env.objc.borrow::<NSOutputStreamHostObject>(this).backing
            {
                Some(buffer.clone())
            } else {
                None
            };

            if let Some(bytes) = buffer_clone {
                let len = bytes.len() as NSUInteger;
                let buf: MutVoidPtr = env.mem.alloc(len as u32).cast();
                env.mem.bytes_at_mut(buf.cast(), len as u32).copy_from_slice(&bytes);
                let data: id = msg_class![env; NSData dataWithBytesNoCopy:buf length:len];
                return data;
            }
            nil
        }
        NSStreamFileCurrentOffsetKey => {
            let offset = if let OutputStreamBacking::Memory { buffer } =
                &env.objc.borrow::<NSOutputStreamHostObject>(this).backing
            {
                buffer.len() as i64
            } else {
                0i64
            };
            msg_class![env; NSNumber numberWithLongLong:offset]
        }
        _ => nil,
    }
}

- (bool)setProperty:(id)_value forKey:(id)_key {
    false
}

- (())scheduleInRunLoop:(id)_run_loop forMode:(id)_mode {
    log_dbg!("NSOutputStream scheduleInRunLoop:forMode: stubbed");
}

- (())removeFromRunLoop:(id)_run_loop forMode:(id)_mode {
    log_dbg!("NSOutputStream removeFromRunLoop:forMode: stubbed");
}

@end

};
