/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSHTTPCookieStorage`.
//!
//! Per Apple's documentation:
//! https://developer.apple.com/documentation/foundation/nshttpcookiestorage
//!
//! Only an empty cookie jar is modelled: cookies received from the network are
//! not persisted, but apps that touch the shared storage during boot (to clear
//! stale cookies, query the accept policy, or persist a logout) keep working.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, retain, HostObject, NSZonePtr};

#[derive(Default)]
pub struct State {
    /// Singleton object (lazily created).
    pub shared: id,
}

#[derive(Default)]
struct NSHTTPCookieStorageHostObject {
    /// NSHTTPCookieAcceptPolicyAlways = 0, Never = 1,
    /// OnlyFromMainDocumentDomain = 2.
    accept_policy: NSInteger,
    /// Retained NSHTTPCookie objects handed to `-setCookie:` and friends.
    stored: Vec<id>,
}
impl HostObject for NSHTTPCookieStorageHostObject {}

pub const CLASSES: crate::objc::ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSHTTPCookieStorage: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSHTTPCookieStorageHostObject {
        accept_policy: 0,
        stored: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)sharedHTTPCookieStorage {
    let existing = env.framework_state.foundation.ns_http_cookie_storage.shared;
    if existing != nil {
        return existing;
    }
    let new_instance: id = msg![env; this alloc];
    let new_instance: id = msg![env; new_instance init];
    env.framework_state.foundation.ns_http_cookie_storage.shared = new_instance;
    new_instance
}

- (NSInteger)cookieAcceptPolicy {
    env.objc.borrow::<NSHTTPCookieStorageHostObject>(this).accept_policy
}

- (())setCookieAcceptPolicy:(NSInteger)policy {
    env.objc.borrow_mut::<NSHTTPCookieStorageHostObject>(this).accept_policy = policy;
}

- (id)cookies {
    msg_class![env; NSMutableArray array]
}

- (id)cookiesForURL:(id)_url {
    nil
}

- (())setCookie:(id)cookie {
    if cookie == nil {
        return;
    }
    retain(env, cookie);
    env.objc.borrow_mut::<NSHTTPCookieStorageHostObject>(this).stored.push(cookie);
}

- (())setCookies:(id)cookies forURL:(id)_url mainDocumentURL:(id)_main {
    if cookies == nil {
        return;
    }
    let count: crate::frameworks::foundation::NSUInteger = msg![env; cookies count];
    for i in 0..count {
        let cookie: id = msg![env; cookies objectAtIndex:i];
        retain(env, cookie);
        env.objc.borrow_mut::<NSHTTPCookieStorageHostObject>(this).stored.push(cookie);
    }
}

- (())deleteCookie:(id)cookie {
    if cookie == nil {
        return;
    }
    let index = env
        .objc
        .borrow::<NSHTTPCookieStorageHostObject>(this)
        .stored
        .iter()
        .position(|&c| c == cookie);
    if let Some(index) = index {
        let removed = env
            .objc
            .borrow_mut::<NSHTTPCookieStorageHostObject>(this)
            .stored
            .remove(index);
        release(env, removed);
    }
}

- (())dealloc {
    let stored = std::mem::take(
        &mut env.objc.borrow_mut::<NSHTTPCookieStorageHostObject>(this).stored,
    );
    for cookie in stored {
        release(env, cookie);
    }
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};
