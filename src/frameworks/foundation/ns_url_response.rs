/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLResponse` and `NSHTTPURLResponse`.
//!
//! These classes represent URL loading system responses. `NSHTTPURLResponse`
//! provides HTTP-specific metadata (status code, headers). See Apple docs:
//! <https://developer.apple.com/documentation/foundation/nsurlresponse>
//! <https://developer.apple.com/documentation/foundation/nshttpurlresponse>

use crate::frameworks::foundation::ns_string::from_rust_string;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

/// Host object for NSURLResponse
#[derive(Default)]
struct NSURLResponseHostObject {
    /// The URL of the response (NSURL*, retained)
    url: id,
    /// MIME type (NSString*, retained)
    mime_type: id,
    /// Expected content length (-1 if unknown)
    expected_content_length: i64,
    /// Text encoding name (NSString*, retained, may be nil)
    text_encoding_name: id,
}
impl HostObject for NSURLResponseHostObject {}

/// Host object for NSHTTPURLResponse (extends NSURLResponse)
#[derive(Default)]
struct NSHTTPURLResponseHostObject {
    /// Base response fields
    url: id,
    mime_type: id,
    expected_content_length: i64,
    text_encoding_name: id,
    /// HTTP status code
    status_code: NSInteger,
    /// HTTP header fields (NSDictionary*, retained)
    all_header_fields: id,
}
impl HostObject for NSHTTPURLResponseHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLResponse: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(NSURLResponseHostObject {
        url: nil,
        mime_type: nil,
        expected_content_length: -1,
        text_encoding_name: nil,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)initWithURL:(id)url
         MIMEType:(id)mime_type
         expectedContentLength:(NSInteger)length
         textEncodingName:(id)encoding_name {
    retain(env, url);
    retain(env, mime_type);
    retain(env, encoding_name);
    let host = env.objc.borrow_mut::<NSURLResponseHostObject>(this);
    host.url = url;
    host.mime_type = mime_type;
    host.expected_content_length = length as i64;
    host.text_encoding_name = encoding_name;
    this
}

- (id)URL {
    env.objc.borrow::<NSURLResponseHostObject>(this).url
}

- (id)MIMEType {
    env.objc.borrow::<NSURLResponseHostObject>(this).mime_type
}

- (i64)expectedContentLength {
    env.objc.borrow::<NSURLResponseHostObject>(this).expected_content_length
}

- (id)textEncodingName {
    env.objc.borrow::<NSURLResponseHostObject>(this).text_encoding_name
}

- (())dealloc {
    let host = env.objc.borrow::<NSURLResponseHostObject>(this);
    let url = host.url;
    let mime = host.mime_type;
    let enc = host.text_encoding_name;
    release(env, url);
    release(env, mime);
    release(env, enc);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

@implementation NSHTTPURLResponse: NSURLResponse

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(NSHTTPURLResponseHostObject {
        url: nil,
        mime_type: nil,
        expected_content_length: -1,
        text_encoding_name: nil,
        status_code: 0,
        all_header_fields: nil,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

// Apple's designated initializer:
// - (instancetype)initWithURL:(NSURL *)url
//                  statusCode:(NSInteger)statusCode
//                 HTTPVersion:(NSString *)HTTPVersion
//                headerFields:(NSDictionary<NSString *,NSString *> *)headerFields;
- (id)initWithURL:(id)url
       statusCode:(NSInteger)status_code
      HTTPVersion:(id)_http_version
     headerFields:(id)header_fields {
    retain(env, url);
    retain(env, header_fields);

    // Derive MIME type from Content-Type header if available
    let mime_type = if header_fields != nil {
        let ct_key = from_rust_string(env, "Content-Type".to_string());
        let ct_val: id = msg![env; header_fields objectForKey:ct_key];
        release(env, ct_key);
        if ct_val != nil {
            retain(env, ct_val);
            ct_val
        } else {
            nil
        }
    } else {
        nil
    };

    // Derive content length from Content-Length header if available
    let content_length: i64 = if header_fields != nil {
        let cl_key = from_rust_string(env, "Content-Length".to_string());
        let cl_val: id = msg![env; header_fields objectForKey:cl_key];
        release(env, cl_key);
        if cl_val != nil {
            let cl_str = crate::frameworks::foundation::ns_string::to_rust_string(env, cl_val);
            cl_str.parse::<i64>().unwrap_or(-1)
        } else {
            -1
        }
    } else {
        -1
    };

    let host = env.objc.borrow_mut::<NSHTTPURLResponseHostObject>(this);
    host.url = url;
    host.mime_type = mime_type;
    host.expected_content_length = content_length;
    host.text_encoding_name = nil;
    host.status_code = status_code;
    host.all_header_fields = header_fields;
    this
}

- (NSInteger)statusCode {
    env.objc.borrow::<NSHTTPURLResponseHostObject>(this).status_code
}

- (id)allHeaderFields {
    env.objc.borrow::<NSHTTPURLResponseHostObject>(this).all_header_fields
}

- (id)URL {
    env.objc.borrow::<NSHTTPURLResponseHostObject>(this).url
}

- (id)MIMEType {
    env.objc.borrow::<NSHTTPURLResponseHostObject>(this).mime_type
}

- (i64)expectedContentLength {
    env.objc.borrow::<NSHTTPURLResponseHostObject>(this).expected_content_length
}

- (id)textEncodingName {
    env.objc.borrow::<NSHTTPURLResponseHostObject>(this).text_encoding_name
}

- (())dealloc {
    let host = env.objc.borrow::<NSHTTPURLResponseHostObject>(this);
    let url = host.url;
    let mime = host.mime_type;
    let enc = host.text_encoding_name;
    let headers = host.all_header_fields;
    release(env, url);
    release(env, mime);
    release(env, enc);
    release(env, headers);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};
