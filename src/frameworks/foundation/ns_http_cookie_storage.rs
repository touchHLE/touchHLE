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
//! In touchHLE we don't actually do any networking. Games that touch the
//! shared cookie storage during boot (for example, to opportunistically clear
//! stale cookies, query the accept policy, or persist a logout) would crash
//! before this class existed. We expose a real Objective-C object whose
//! behaviour matches Apple's contract for an *empty* cookie jar:
//!
//!  * `+sharedHTTPCookieStorage` always returns the same instance.
//!  * `-cookies` always returns an empty (mutable) array.
//!  * `-cookiesForURL:` returns `nil` (Apple docs: "the cookies for the URL,
//!    or `nil` if no cookies have been received for the URL").
//!  * `-setCookie:` / `-setCookies:forURL:mainDocumentURL:` retain the
//!    cookies so the caller's autorelease pool doesn't double-release them.
//!  * `-deleteCookie:` is a no-op (we never deliver real cookies anyway).
//!  * `-cookieAcceptPolicy` / `-setCookieAcceptPolicy:` round-trip the
//!    policy as Apple documents.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, retain, HostObject, NSZonePtr};

#[derive(Default)]
pub struct State {
    /// Singleton object (lazy).
    shared: id,
}

#[derive(Default)]
struct NSHTTPCookieStorageHostObject {
    /// Cookie accept policy:
    ///   NSHTTPCookieAcceptPolicyAlways = 0
    ///   NSHTTPCookieAcceptPolicyNever = 1
    ///   NSHTTPCookieAcceptPolicyOnlyFromMainDocumentDomain = 2
    accept_policy: NSInteger,
    /// Retained NSHTTPCookie objects so callers can keep an unbalanced
    /// reference after `-setCookie:`.
    stored: Vec<id>,
}
impl HostObject for NSHTTPCookieStorageHostObject {}

pub const CLASSES: crate::objc::ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSHTTPCookieStorage: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSHTTPCookieStorageHostObject {
        accept_policy: 0, // NSHTTPCookieAcceptPolicyAlways
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

// `-cookies` — return an empty NSArray (we never receive cookies). Per the
// Apple docs we must return an `NSArray *`, not `nil`.
- (id)cookies {
    msg_class![env; NSMutableArray array]
}

// Per Apple docs: returns the cookies for a URL or `nil` if none. Since we
// never deliver any cookies to the caller, `nil` is the documented answer.
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
    if cookies == nil { return; }
    let count: crate::frameworks::foundation::NSUInteger = msg![env; cookies count];
    for i in 0..count {
        let c: id = msg![env; cookies objectAtIndex:i];
        retain(env, c);
        env.objc.borrow_mut::<NSHTTPCookieStorageHostObject>(this).stored.push(c);
    }
}

- (())deleteCookie:(id)cookie {
    if cookie == nil { return; }
    let idx_opt = {
        let host = env.objc.borrow::<NSHTTPCookieStorageHostObject>(this);
        host.stored.iter().position(|&c| c == cookie)
    };
    if let Some(idx) = idx_opt {
        let removed = env
            .objc
            .borrow_mut::<NSHTTPCookieStorageHostObject>(this)
            .stored
            .remove(idx);
        release(env, removed);
    }
}

- (())dealloc {
    let stored = std::mem::take(&mut env.objc.borrow_mut::<NSHTTPCookieStorageHostObject>(this).stored);
    for c in stored {
        release(env, c);
    }
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};
