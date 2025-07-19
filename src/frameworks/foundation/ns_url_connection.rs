/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLConnection` mock implementation.

use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
pub struct NSURLConnectionHostObject {
    pub request_url: Option<String>,
    pub is_running: bool,
}

impl HostObject for NSURLConnectionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLConnection: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSURLConnectionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithRequest:(id)request delegate:(id)delegate startImmediately:(bool)start {
    log!("[NSURLConnection initWithRequest:delegate:startImmediately:] => request={:?}, delegate={:?}, start={:?}", request, delegate, start);

    let url: Option<String> = Some("<mocked URL>".to_string());
    let host = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
    host.request_url = url;
    host.is_running = start;

    this
}

- (())start {
    log!("[NSURLConnection start]");
    env.objc.borrow_mut::<NSURLConnectionHostObject>(this).is_running = true;
}

- (())cancel {
    log!("[NSURLConnection cancel]");
    env.objc.borrow_mut::<NSURLConnectionHostObject>(this).is_running = false;
}

@end

};
