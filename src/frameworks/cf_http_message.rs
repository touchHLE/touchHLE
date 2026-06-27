/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(non_snake_case, non_upper_case_globals)]

//! `CFHTTPMessage` — CFNetwork HTTP request / response builder.
//!
//! Implements the subset of `<CFNetwork/CFHTTPMessage.h>` documented by
//! Apple:
//!
//! ```c
//! CFHTTPMessageRef CFHTTPMessageCreateRequest (CFAllocatorRef, CFStringRef method,
//!                                              CFURLRef url, CFStringRef httpVersion);
//! CFHTTPMessageRef CFHTTPMessageCreateResponse(CFAllocatorRef, CFIndex statusCode,
//!                                              CFStringRef statusDescription,
//!                                              CFStringRef httpVersion);
//! CFHTTPMessageRef CFHTTPMessageCreateEmpty   (CFAllocatorRef, Boolean isRequest);
//! CFHTTPMessageRef CFHTTPMessageCreateCopy    (CFAllocatorRef, CFHTTPMessageRef);
//!
//! void       CFHTTPMessageSetHeaderFieldValue (CFHTTPMessageRef,
//!                                              CFStringRef name,
//!                                              CFStringRef value);
//! CFStringRef CFHTTPMessageCopyHeaderFieldValue(CFHTTPMessageRef, CFStringRef);
//! CFDictionaryRef CFHTTPMessageCopyAllHeaderFields(CFHTTPMessageRef);
//!
//! void  CFHTTPMessageSetBody (CFHTTPMessageRef, CFDataRef bodyData);
//! CFDataRef CFHTTPMessageCopyBody(CFHTTPMessageRef);
//! Boolean  CFHTTPMessageAppendBytes(CFHTTPMessageRef, const UInt8*, CFIndex);
//! Boolean  CFHTTPMessageIsHeaderComplete(CFHTTPMessageRef);
//!
//! CFDataRef    CFHTTPMessageCopySerializedMessage(CFHTTPMessageRef);
//! Boolean      CFHTTPMessageIsRequest          (CFHTTPMessageRef);
//! CFIndex      CFHTTPMessageGetResponseStatusCode(CFHTTPMessageRef);
//! CFStringRef  CFHTTPMessageCopyRequestMethod  (CFHTTPMessageRef);
//! CFURLRef     CFHTTPMessageCopyRequestURL    (CFHTTPMessageRef);
//! CFStringRef  CFHTTPMessageCopyVersion        (CFHTTPMessageRef);
//! ```
//!
//! Messages are stored as a real CFType-backed Objective-C object so they
//! participate in normal retain/release accounting and can be passed across
//! CFNetwork APIs without crashing. `CFHTTPMessageCopySerializedMessage`
//! produces a properly framed `request-line + CRLF-delimited headers + body`
//! byte buffer per [RFC
//! 7230](https://datatracker.ietf.org/doc/html/rfc7230#section-3) so that
//! consumers like `ASIHTTPRequest` that write it to a socket get a real HTTP
//! message instead of zeroed memory.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use crate::frameworks::core_foundation::cf_data::{CFDataCreate, CFDataGetBytePtr, CFDataGetLength};
use crate::frameworks::core_foundation::cf_string::CFStringRef;
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::core_foundation::{CFIndex, CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::mem::{ConstPtr, MutPtr};
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, retain, release, ClassExports, HostObject,
};
use crate::Environment;

pub type CFHTTPMessageRef = CFTypeRef;

/// HTTP method/version/url/headers/body container. Headers preserve insertion
/// order (per RFC 7230 §3.2.2 the order of header fields with differing
/// field-names is not significant, but ordering matters for repeated fields
/// such as `Set-Cookie`, so we use a `Vec<(name, value)>` rather than a
/// hash map).
#[derive(Default)]
struct CFHTTPMessageHostObject {
    is_request: bool,
    /// e.g. "GET", "POST". Always upper-case ASCII.
    method: String,
    /// e.g. "HTTP/1.1". Defaults to "HTTP/1.1".
    version: String,
    /// Request-line URL string (Request-URI) — for requests.
    request_url: String,
    /// Response status code — for responses (0 otherwise).
    status_code: CFIndex,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    header_complete: bool,
}
impl HostObject for CFHTTPMessageHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CFHTTPMessage: NSObject

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

fn validate_allocator(env: &mut Environment, allocator: CFAllocatorRef) -> bool {
    allocator == kCFAllocatorDefault
        || allocator.is_null()
        || env.mem.read(allocator).is_system_default()
}

fn alloc_message(env: &mut Environment, msg: CFHTTPMessageHostObject) -> CFHTTPMessageRef {
    let class = env
        .objc
        .get_known_class("_touchHLE_CFHTTPMessage", &mut env.mem);
    env.objc.alloc_object(class, Box::new(msg), &mut env.mem)
}

// MARK: - Retain / Release

pub fn CFHTTPMessageRetain(env: &mut Environment, msg: CFHTTPMessageRef) -> CFHTTPMessageRef {
    if !msg.is_null() {
        CFRetain(env, msg)
    } else {
        msg
    }
}

pub fn CFHTTPMessageRelease(env: &mut Environment, msg: CFHTTPMessageRef) {
    if !msg.is_null() {
        CFRelease(env, msg);
    }
}

// MARK: - Constructors

fn version_string(env: &mut Environment, version: CFStringRef) -> String {
    if version.is_null() {
        "HTTP/1.1".to_string()
    } else {
        let v = to_rust_string(env, version).into_owned();
        if v.is_empty() { "HTTP/1.1".to_string() } else { v }
    }
}

fn CFHTTPMessageCreateRequest(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    method: CFStringRef,
    url: CFURLRef,
    http_version: CFStringRef,
) -> CFHTTPMessageRef {
    if !validate_allocator(env, allocator) {
        return nil;
    }
    let method_str = if method.is_null() {
        "GET".to_string()
    } else {
        to_rust_string(env, method).into_owned()
    };
    let url_str = if url.is_null() {
        String::new()
    } else {
        // CFURLGetString is toll-free bridged with [NSURL absoluteString];
        // use that to avoid pulling in cf_url internals.
        let s: id = msg![env; url absoluteString];
        if s == nil {
            String::new()
        } else {
            to_rust_string(env, s).into_owned()
        }
    };
    let version_str = version_string(env, http_version);
    alloc_message(
        env,
        CFHTTPMessageHostObject {
            is_request: true,
            method: method_str,
            version: version_str,
            request_url: url_str,
            status_code: 0,
            headers: Vec::new(),
            body: Vec::new(),
            header_complete: false,
        },
    )
}

fn CFHTTPMessageCreateResponse(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    status_code: CFIndex,
    _status_description: CFStringRef,
    http_version: CFStringRef,
) -> CFHTTPMessageRef {
    if !validate_allocator(env, allocator) {
        return nil;
    }
    let version_str = version_string(env, http_version);
    alloc_message(
        env,
        CFHTTPMessageHostObject {
            is_request: false,
            method: String::new(),
            version: version_str,
            request_url: String::new(),
            status_code,
            headers: Vec::new(),
            body: Vec::new(),
            header_complete: false,
        },
    )
}

fn CFHTTPMessageCreateEmpty(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    is_request: bool,
) -> CFHTTPMessageRef {
    if !validate_allocator(env, allocator) {
        return nil;
    }
    alloc_message(
        env,
        CFHTTPMessageHostObject {
            is_request,
            method: String::new(),
            version: "HTTP/1.1".to_string(),
            request_url: String::new(),
            status_code: 0,
            headers: Vec::new(),
            body: Vec::new(),
            header_complete: false,
        },
    )
}

fn CFHTTPMessageCreateCopy(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    src: CFHTTPMessageRef,
) -> CFHTTPMessageRef {
    if src.is_null() {
        return nil;
    }
    if !validate_allocator(env, allocator) {
        return nil;
    }
    let copy = {
        let host = env.objc.borrow::<CFHTTPMessageHostObject>(src);
        CFHTTPMessageHostObject {
            is_request: host.is_request,
            method: host.method.clone(),
            version: host.version.clone(),
            request_url: host.request_url.clone(),
            status_code: host.status_code,
            headers: host.headers.clone(),
            body: host.body.clone(),
            header_complete: host.header_complete,
        }
    };
    alloc_message(env, copy)
}

// MARK: - Headers

fn CFHTTPMessageSetHeaderFieldValue(
    env: &mut Environment,
    msg: CFHTTPMessageRef,
    name: CFStringRef,
    value: CFStringRef,
) {
    if msg.is_null() || name.is_null() {
        return;
    }
    let name_str = to_rust_string(env, name).into_owned();
    // Per Apple's docs, passing NULL `value` removes the header.
    let new_value: Option<String> = if value.is_null() {
        None
    } else {
        Some(to_rust_string(env, value).into_owned())
    };
    let host = env.objc.borrow_mut::<CFHTTPMessageHostObject>(msg);
    host.headers
        .retain(|(n, _)| !n.eq_ignore_ascii_case(&name_str));
    if let Some(v) = new_value {
        host.headers.push((name_str, v));
    }
}

fn CFHTTPMessageCopyHeaderFieldValue(
    env: &mut Environment,
    msg: CFHTTPMessageRef,
    name: CFStringRef,
) -> CFStringRef {
    if msg.is_null() || name.is_null() {
        return nil;
    }
    let name_str = to_rust_string(env, name).into_owned();
    let value = {
        let host = env.objc.borrow::<CFHTTPMessageHostObject>(msg);
        host.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&name_str))
            .map(|(_, v)| v.clone())
    };
    match value {
        Some(v) => from_rust_string(env, v),
        None => nil,
    }
}

fn CFHTTPMessageCopyAllHeaderFields(env: &mut Environment, msg: CFHTTPMessageRef) -> CFTypeRef {
    if msg.is_null() {
        return nil;
    }
    let pairs: Vec<(String, String)> = env
        .objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .headers
        .clone();
    let dict: id = msg_class![env; NSMutableDictionary dictionary];
    for (k, v) in pairs {
        let key: id = from_rust_string(env, k);
        let val: id = from_rust_string(env, v);
        () = msg![env; dict setObject:val forKey:key];
        release(env, key);
        release(env, val);
    }
    retain(env, dict)
}

// MARK: - Body

fn CFHTTPMessageSetBody(env: &mut Environment, msg: CFHTTPMessageRef, body_data: CFTypeRef) {
    if msg.is_null() {
        return;
    }
    let bytes = if body_data.is_null() {
        Vec::new()
    } else {
        let len = CFDataGetLength(env, body_data) as usize;
        let ptr: ConstPtr<u8> = CFDataGetBytePtr(env, body_data);
        if ptr.is_null() || len == 0 {
            Vec::new()
        } else {
            env.mem.bytes_at(ptr, len as u32).to_vec()
        }
    };
    let host = env.objc.borrow_mut::<CFHTTPMessageHostObject>(msg);
    host.body = bytes;
}

fn CFHTTPMessageCopyBody(env: &mut Environment, msg: CFHTTPMessageRef) -> CFTypeRef {
    if msg.is_null() {
        return nil;
    }
    let bytes = env
        .objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .body
        .clone();
    if bytes.is_empty() {
        return nil;
    }
    // CFDataCreate copies the bytes; we need to stage them in guest memory
    // first.
    let len = bytes.len() as u32;
    let buf: MutPtr<u8> = env.mem.alloc(len).cast();
    env.mem
        .bytes_at_mut(buf, len)
        .copy_from_slice(&bytes);
    let data: CFTypeRef = CFDataCreate(env, kCFAllocatorDefault, buf.cast_const(), len as CFIndex);
    env.mem.free(buf.cast());
    data
}

fn CFHTTPMessageAppendBytes(
    env: &mut Environment,
    msg: CFHTTPMessageRef,
    new_bytes: ConstPtr<u8>,
    new_bytes_len: CFIndex,
) -> bool {
    if msg.is_null() || new_bytes.is_null() || new_bytes_len <= 0 {
        return false;
    }
    let slice = env
        .mem
        .bytes_at(new_bytes, new_bytes_len as u32)
        .to_vec();

    // The bytes are an on-the-wire HTTP message that the caller is feeding
    // us in chunks. We parse what we can: everything up to the first
    // double-CRLF becomes the header block; anything that follows is
    // appended to the body.
    let host = env.objc.borrow_mut::<CFHTTPMessageHostObject>(msg);
    if host.header_complete {
        host.body.extend_from_slice(&slice);
        return true;
    }

    // Accumulate into a scratch buffer (`body`, repurposed until we finish
    // parsing the head — this matches CFNetwork's behaviour of pre-parse
    // accumulation in the response case).
    host.body.extend_from_slice(&slice);
    if let Some(end) = find_header_end(&host.body) {
        let head: Vec<u8> = host.body.drain(..end).collect();
        // Drop the trailing CRLF CRLF (or LF LF) that the find returned.
        let drop = if host.body.starts_with(b"\r\n") {
            2
        } else if host.body.starts_with(b"\n") {
            1
        } else {
            0
        };
        host.body.drain(..drop);
        let text = String::from_utf8_lossy(&head).into_owned();
        parse_head_into(host, &text);
        host.header_complete = true;
    }
    true
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    // Returns the length of the request/status line + headers (i.e. the
    // index of the first CRLF/LF that *separates* head from body).
    for i in 0..buf.len() {
        if buf[i..].starts_with(b"\r\n\r\n") {
            return Some(i + 2);
        }
        if buf[i..].starts_with(b"\n\n") {
            return Some(i + 1);
        }
    }
    None
}

fn parse_head_into(host: &mut CFHTTPMessageHostObject, text: &str) {
    let mut lines = text.lines();
    if let Some(first) = lines.next() {
        if host.is_request {
            // METHOD SP REQUEST-URI SP HTTP-VERSION
            let mut parts = first.splitn(3, ' ');
            if let (Some(m), Some(u), Some(v)) = (parts.next(), parts.next(), parts.next()) {
                host.method = m.to_string();
                host.request_url = u.to_string();
                host.version = v.to_string();
            }
        } else {
            // HTTP-VERSION SP STATUS-CODE SP REASON-PHRASE
            let mut parts = first.splitn(3, ' ');
            if let (Some(v), Some(c), _) = (parts.next(), parts.next(), parts.next()) {
                host.version = v.to_string();
                if let Ok(n) = c.parse::<i32>() {
                    host.status_code = n;
                }
            }
        }
    }
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, val)) = line.split_once(':') {
            host.headers
                .push((name.trim().to_string(), val.trim().to_string()));
        }
    }
}

fn CFHTTPMessageIsHeaderComplete(env: &mut Environment, msg: CFHTTPMessageRef) -> bool {
    if msg.is_null() {
        return false;
    }
    env.objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .header_complete
}

// MARK: - Inspection

fn CFHTTPMessageIsRequest(env: &mut Environment, msg: CFHTTPMessageRef) -> bool {
    if msg.is_null() {
        return false;
    }
    env.objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .is_request
}

fn CFHTTPMessageGetResponseStatusCode(env: &mut Environment, msg: CFHTTPMessageRef) -> CFIndex {
    if msg.is_null() {
        return 0;
    }
    env.objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .status_code
}

fn CFHTTPMessageCopyRequestMethod(env: &mut Environment, msg: CFHTTPMessageRef) -> CFStringRef {
    if msg.is_null() {
        return nil;
    }
    let m = env
        .objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .method
        .clone();
    if m.is_empty() {
        return nil;
    }
    from_rust_string(env, m)
}

fn CFHTTPMessageCopyRequestURL(env: &mut Environment, msg: CFHTTPMessageRef) -> CFURLRef {
    if msg.is_null() {
        return nil;
    }
    let u = env
        .objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .request_url
        .clone();
    if u.is_empty() {
        return nil;
    }
    let s: id = from_rust_string(env, u);
    let url: id = msg_class![env; NSURL URLWithString:s];
    release(env, s);
    if url == nil {
        return nil;
    }
    retain(env, url)
}

fn CFHTTPMessageCopyVersion(env: &mut Environment, msg: CFHTTPMessageRef) -> CFStringRef {
    if msg.is_null() {
        return nil;
    }
    let v = env
        .objc
        .borrow::<CFHTTPMessageHostObject>(msg)
        .version
        .clone();
    from_rust_string(env, v)
}

// MARK: - Serialisation

fn CFHTTPMessageCopySerializedMessage(env: &mut Environment, msg: CFHTTPMessageRef) -> CFTypeRef {
    if msg.is_null() {
        return nil;
    }
    let bytes = {
        let host = env.objc.borrow::<CFHTTPMessageHostObject>(msg);
        let mut out = Vec::with_capacity(256 + host.body.len());
        if host.is_request {
            out.extend_from_slice(host.method.as_bytes());
            out.push(b' ');
            // RFC 7230 §5.3: the path part of the URL is the request-target.
            // We don't try to split scheme://host/path here because CFNetwork
            // accepts either form (it normalises later via the connection
            // pool).
            if host.request_url.is_empty() {
                out.push(b'/');
            } else {
                out.extend_from_slice(host.request_url.as_bytes());
            }
            out.push(b' ');
            out.extend_from_slice(host.version.as_bytes());
        } else {
            out.extend_from_slice(host.version.as_bytes());
            out.push(b' ');
            out.extend_from_slice(host.status_code.to_string().as_bytes());
            out.push(b' ');
            // Status reason phrase isn't tracked; emit a conventional one.
            out.extend_from_slice(status_reason(host.status_code).as_bytes());
        }
        out.extend_from_slice(b"\r\n");
        for (k, v) in &host.headers {
            out.extend_from_slice(k.as_bytes());
            out.extend_from_slice(b": ");
            out.extend_from_slice(v.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&host.body);
        out
    };

    let len = bytes.len() as u32;
    let buf: MutPtr<u8> = env.mem.alloc(len.max(1)).cast();
    if len > 0 {
        env.mem
            .bytes_at_mut(buf, len)
            .copy_from_slice(&bytes);
    }
    let data: CFTypeRef = CFDataCreate(env, kCFAllocatorDefault, buf.cast_const(), len as CFIndex);
    env.mem.free(buf.cast());
    data
}

fn status_reason(code: CFIndex) -> &'static str {
    match code {
        100 => "Continue",
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFHTTPMessageRetain(_)),
    export_c_func!(CFHTTPMessageRelease(_)),
    export_c_func!(CFHTTPMessageCreateRequest(_, _, _, _)),
    export_c_func!(CFHTTPMessageCreateResponse(_, _, _, _)),
    export_c_func!(CFHTTPMessageCreateEmpty(_, _)),
    export_c_func!(CFHTTPMessageCreateCopy(_, _)),
    export_c_func!(CFHTTPMessageSetHeaderFieldValue(_, _, _)),
    export_c_func!(CFHTTPMessageCopyHeaderFieldValue(_, _)),
    export_c_func!(CFHTTPMessageCopyAllHeaderFields(_)),
    export_c_func!(CFHTTPMessageSetBody(_, _)),
    export_c_func!(CFHTTPMessageCopyBody(_)),
    export_c_func!(CFHTTPMessageAppendBytes(_, _, _)),
    export_c_func!(CFHTTPMessageIsHeaderComplete(_)),
    export_c_func!(CFHTTPMessageIsRequest(_)),
    export_c_func!(CFHTTPMessageGetResponseStatusCode(_)),
    export_c_func!(CFHTTPMessageCopyRequestMethod(_)),
    export_c_func!(CFHTTPMessageCopyRequestURL(_)),
    export_c_func!(CFHTTPMessageCopyVersion(_)),
    export_c_func!(CFHTTPMessageCopySerializedMessage(_)),
];
