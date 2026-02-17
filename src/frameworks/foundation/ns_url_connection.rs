/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLConnection`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use crate::frameworks::foundation::ns_run_loop::NSDefaultRunLoopMode;
use crate::frameworks::foundation::ns_string::{self, to_rust_string};
use crate::frameworks::foundation::ns_url_request::{self, NSURLRequestCachePolicy};
use crate::frameworks::foundation::NSTimeInterval;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

#[derive(Default, Debug, PartialEq, Eq)]
enum ScheduledIn {
    #[default]
    Nowhere,
    RunLoop,
    OperationQueue,
}

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone)]
enum ConnectionState {
    #[default]
    NotStarted,
    RequestSent,
    Completed(id), // NSURLResponse *
}

#[derive(Default)]
pub struct NSURLConnectionHostObject {
    original_request: id,
    current_request: id,
    delegate: id,
    tcp_stream: Option<TcpStream>,
    scheduled_in: ScheduledIn,
    state: ConnectionState,
}
impl HostObject for NSURLConnectionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLConnection: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::new(NSURLConnectionHostObject::default()), &mut env.mem)
}

+ (id)connectionWithRequest:(id)request // NSURLRequest *
                   delegate:(id)delegate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRequest:request delegate:delegate];
    autorelease(env, new)
}

- (id)initWithRequest:(id)request // NSURLRequest *
             delegate:(id)delegate {
    msg![env; this initWithRequest:request delegate:delegate startImmediately:true]
}

- (id)initWithRequest:(id)request // NSURLRequest *
             delegate:(id)delegate
     startImmediately:(bool)start_immediately {
    let request_copy: id = msg_class![env; NSMutableURLRequest new];
    // Copy the request's properties to the mutable request
    let request_url: id = msg![env; request URL];
    () = msg![env; request_copy setURL:request_url];
    let request_cache_policy: NSURLRequestCachePolicy = msg![env; request cachePolicy];
    () = msg![env; request_copy setCachePolicy:request_cache_policy];
    let request_timeout_interval: NSTimeInterval = msg![env; request timeoutInterval];
    () = msg![env; request_copy setTimeoutInterval:request_timeout_interval];
    let request_http_method: id = msg![env; request HTTPMethod];
    () = msg![env; request_copy setHTTPMethod:request_http_method];
    let request_http_body: id = msg![env; request HTTPBody];
    () = msg![env; request_copy setHTTPBody:request_http_body];
    let request_http_header_fields: id = msg![env; request allHTTPHeaderFields];
    let request_http_header_fields_copy: id = msg_class![env; NSMutableDictionary dictionaryWithDictionary:request_http_header_fields];
    ns_url_request::replace_all_http_header_fields(env, request_copy, request_http_header_fields_copy);
    // TODO: Handle other properties

    let host_object = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
    host_object.original_request = request_copy;
    host_object.current_request = request;
    host_object.delegate = delegate;
    retain(env, request);
    retain(env, request_copy);

    let return_value: id = if env.options.network_access {
        let description = msg![env; request_url description];
        let description = to_rust_string(env, description);
        log_dbg!("Opening NSURLConnection to {}", description);
        let hostname = msg![env; request_url host];
        let hostname = to_rust_string(env, hostname);
        let port = msg![env; request_url port];
        let port: u16 = if port == nil {
            let scheme = msg![env; request_url scheme];
            let scheme = to_rust_string(env, scheme);
            match &*scheme {
                "http" => 80,
                "https" => 443,
                _ => panic!("Unknown schema: {}", scheme)
            }
        } else {
            msg![env; port unsignedShortValue]
        };
        let tcp_destination = format!("{}:{}", hostname, port);
        log_dbg!("Connecting to {}", tcp_destination);
        if let Ok(tcp_stream) = TcpStream::connect(tcp_destination) {
            retain(env, delegate);
            env.objc.borrow_mut::<NSURLConnectionHostObject>(this).tcp_stream = Some(tcp_stream);
            if start_immediately {
                () = msg![env; this start];
            }
            this
        } else {
            nil
        }
    } else {
        log_dbg!("Network access is disabled");
        nil
    };
    log_dbg!(
        "[(NSURLConnection *){:?} initWithRequest:{:?} delegate:{:?} startImmediately:{}] -> {:?}",
        this,
        request,
        delegate,
        start_immediately,
        return_value
    );
    return_value
}

- (())start {
    log_dbg!("[(NSURLConnection *){:?} start]", this);
    let host_object = env.objc.borrow::<NSURLConnectionHostObject>(this);
    assert!(host_object.tcp_stream.is_some());
    assert_eq!(host_object.state, ConnectionState::NotStarted);
    if host_object.scheduled_in == ScheduledIn::Nowhere {
        let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
        let mode: id = ns_string::get_static_str(env, NSDefaultRunLoopMode);
        () = msg![env; this scheduleInRunLoop:run_loop forMode:mode];
    }
}

- (())_touchHLE_runConnection:(id)_user_info {
    // TODO: LOADS of error handling
    // TODO: Send A LOT of callbacks to the delegate

    // HTTP request start line
    let request: id = msg![env; this originalRequest];
    let method: id = msg![env; request HTTPMethod];
    let method = to_rust_string(env, method);
    let url: id = msg![env; request URL];
    let path: id = msg![env; url path];
    let path = to_rust_string(env, path);
    env.objc.borrow::<NSURLConnectionHostObject>(this).tcp_stream.as_ref().unwrap().write_fmt(format_args!("{} {} HTTP/1.1\r\n", method, path)).unwrap();

    // HTTP request headers
    let host = msg![env; url host];
    let host = to_rust_string(env, host);
    env.objc.borrow::<NSURLConnectionHostObject>(this).tcp_stream.as_ref().unwrap().write_fmt(format_args!("Host: {}\r\n", host)).unwrap();
    let body = msg![env; request HTTPBody];
    let content_length = msg![env; body length];
    env.objc.borrow::<NSURLConnectionHostObject>(this).tcp_stream.as_ref().unwrap().write_fmt(format_args!("Content-Length: {}\r\n", content_length)).unwrap();
    let http_header_fields: id = msg![env; request allHTTPHeaderFields];
    // TODO: use keyEnumerator
    let keys_arr: id = msg![env; http_header_fields allKeys];
    let enumerator: id = msg![env; keys_arr objectEnumerator];
    loop {
        let key: id = msg![env; enumerator nextObject];
        if key == nil {
            break;
        }
        let value: id = msg![env; http_header_fields objectForKey:key];
        let key = to_rust_string(env, key);
        let value = to_rust_string(env, value);
        env.objc.borrow::<NSURLConnectionHostObject>(this).tcp_stream.as_ref().unwrap().write_fmt(format_args!("{}: {}\r\n", key, value)).unwrap();
    }
    // Empty line to separate headers from body
    env.objc.borrow::<NSURLConnectionHostObject>(this).tcp_stream.as_ref().unwrap().write_all(b"\r\n").unwrap();

    // HTTP request body
    // TODO: Handle query params
    if body != nil {
        let body_bytes: ConstPtr<u8>  = msg![env; body bytes];
        let body_bytes = env.mem.bytes_at(body_bytes, content_length);
        env.objc.borrow::<NSURLConnectionHostObject>(this).tcp_stream.as_ref().unwrap().write_all(body_bytes).unwrap();
    }

    env.objc.borrow_mut::<NSURLConnectionHostObject>(this).state = ConnectionState::RequestSent;

    // Receive response.
    // TODO: Move to the run loop, do asynchronously.
    let response: id = msg_class![env; NSHTTPURLResponse new];
    retain(env, response);
    // TODO: A lot of things
    let host_object = env.objc.borrow::<NSURLConnectionHostObject>(this);
    let mut tcp_stream = host_object.tcp_stream.as_ref().unwrap();
    let mut reader = BufReader::new(tcp_stream);

    // HTTP response start line
    let mut line = Vec::<u8>::new();
    reader.read_until(b'\n', &mut line).unwrap();
    let line = String::from_utf8(line).unwrap();
    let mut parts = line.splitn(3, " ");
    parts.next().unwrap(); // Skip HTTP version
    let status_code = parts.next().unwrap().parse::<usize>().unwrap();
    // Skip reason phrase
    assert!(matches!(status_code, 200 | 204));

    // HTTP response headers
    let mut content_length = None;
    loop {
        // TODO: Parse headers
        let mut line = Vec::<u8>::new();
        reader.read_until(b'\n', &mut line).unwrap();
        if line == [b'\r', b'\n'] {
            break;
        }
        let line_string = String::from_utf8(line).unwrap();
        if let Some(content_length_string) = line_string.strip_prefix("Content-Length: ") {
            content_length = content_length_string.parse::<usize>().ok();
        }
    }

    // HTTP response body
    let body = if let Some(content_length) = content_length {
        let mut body = Vec::<u8>::with_capacity(content_length);
        tcp_stream.read_exact(&mut body).unwrap();
        body
    } else {
        let mut body = Vec::<u8>::new();
        tcp_stream.read_to_end(&mut body).unwrap();
        body
    };

    let guest_buffer = env.mem.alloc(body.len() as GuestUSize);
    env.mem.bytes_at_mut(guest_buffer.cast(), body.len() as GuestUSize).copy_from_slice(&body);
    let ns_data: id = msg_class![env; NSData dataWithBytesNoCopy:guest_buffer length:(body.len() as GuestUSize)];
    let delegate = env.objc.borrow::<NSURLConnectionHostObject>(this).delegate;
    () = msg![env; delegate connection:this didReceiveData: ns_data];
    // TODO: Set NSHTTPURLResponse's properties
    () = msg![env; delegate connectionDidFinishLoading:this];
    () = msg![env; delegate connection:this didReceiveResponse:response];
    env.objc.borrow_mut::<NSURLConnectionHostObject>(this).state = ConnectionState::Completed(response);
}

- (())scheduleInRunLoop:(id)run_loop // NSRunLoop *
                forMode:(id)mode { // NSString *
    log_dbg!("[(NSURLConnection *){:?} scheduleInRunLoop:{:?} forMode:{:?}]", this, run_loop, mode);
    let host_object = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
    assert_ne!(host_object.scheduled_in, ScheduledIn::OperationQueue);
    host_object.scheduled_in = ScheduledIn::RunLoop;
    let selector = env.objc.lookup_selector("_touchHLE_runConnection:").unwrap();
    let timer:id = msg_class![env; NSTimer timerWithTimeInterval:(0.0)
                                           target:this
                                           selector:selector
                                           userInfo:nil
                                           repeats:false];
    () = msg![env; run_loop addTimer:timer forMode:mode];
}

- (())setDelegateQueue:(id)queue { // NSOperationQueue *
    log!("TODO: [(NSURLConnection *){:?} setDelegateQueue:{:?}]", this, queue);
    let host_object = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
    assert_ne!(host_object.scheduled_in, ScheduledIn::RunLoop);
    host_object.scheduled_in = ScheduledIn::OperationQueue;
    unimplemented!();
}

- (id)originalRequest {
    env.objc.borrow::<NSURLConnectionHostObject>(this).original_request
}

- (id)currentRequest {
    env.objc.borrow::<NSURLConnectionHostObject>(this).current_request
}

- (())dealloc {
    let &NSURLConnectionHostObject {
        original_request,
        current_request,
        state,
        ..
    } = env.objc.borrow(this);
    release(env, original_request);
    release(env, current_request);
    match state {
        ConnectionState::Completed(response) => release(env, response),
        _ => {}
    };
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
