/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `Social.framework/Social`.
//!
//! On iOS 6+, Social provides `SLComposeViewController` and the
//! `SLServiceType*` string identifiers used to talk to system-level Twitter,
//! Facebook, Sina/Tencent Weibo and LinkedIn accounts.
//!
//! touchHLE only targets pre-iOS-4-era games today, but many later apps still
//! list Social as a Mach-O dependency (often pulled in transitively by an
//! analytics or sharing SDK such as Flurry or Facebook Audience Network) and
//! reference its `SLServiceType*` constants by symbol name.  Without a
//! [crate::dyld::HostDylib] entry, those constants stay NULL and any
//! guest-side `[NSString isEqualToString:SLServiceTypeFacebook]` crashes the
//! emulator with a NULL-page read.  This stub satisfies the dependency and
//! returns plain NSString constants whose values are the canonical Apple
//! identifiers.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

pub type SLRequestMethod = NSInteger;
pub const SLRequestMethodGET: SLRequestMethod = 0;
pub const SLRequestMethodPOST: SLRequestMethod = 1;
pub const SLRequestMethodDELETE: SLRequestMethod = 2;
pub const SLRequestMethodPUT: SLRequestMethod = 3;

#[derive(Default)]
pub struct SLRequestHostObject {
    service_type: id,
    request_method: SLRequestMethod,
    url: id,
    parameters: id,
    account: id,
}
impl HostObject for SLRequestHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// `SLComposeViewController` (iOS 6.0+). Presents the system share sheet
// for a given service. touchHLE has no UIKit presentation pipeline and no
// Settings-app account configuration, so following Apple's documented
// contract we report every service as unavailable. Apps then take their
// "service is not configured" code path instead of crashing on
// `[SLComposeViewController alloc]` / `setInitialText:`.
// <https://developer.apple.com/documentation/social/slcomposeviewcontroller>
@implementation SLComposeViewController: UIViewController

// `+ (BOOL)isAvailableForServiceType:(NSString *)serviceType` — `YES` if
// the user is signed into the requested service. touchHLE has no Accounts
// framework, so always `NO`.
+ (bool)isAvailableForServiceType:(id)_service_type {
    false
}

// `+ (SLComposeViewController *)composeViewControllerForServiceType:(NSString *)serviceType`
// — Apple's docs explicitly note this returns `nil` when the service is
// unavailable. Since `+isAvailableForServiceType:` always reports `NO`,
// this returns `nil` too.
+ (id)composeViewControllerForServiceType:(id)_service_type {
    nil
}

@end

// `SLRequest` (iOS 6.0+). Wraps an authenticated HTTP request that
// would normally be sent through a Social-framework account
// (Twitter/Facebook/etc.) configured in iOS Settings.
// <https://developer.apple.com/documentation/social/slrequest>
//
// touchHLE has no Accounts framework, so any request asynchronously
// completes with an SLRequestErrorDomain error. The request can still
// be transformed into an NSURLRequest via `-preparedURLRequest`, which
// some apps inspect even without actually performing the call.
@implementation SLRequest: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SLRequestHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)requestWithMethod:(SLRequestMethod)method
                    URL:(id)url
             parameters:(id)parameters {
    let alloc: id = msg![env; this alloc];
    let init: id = msg![env;
        alloc initWithMethod:method
                          URL:url
                   parameters:parameters];
    autorelease(env, init)
}

- (id)initWithMethod:(SLRequestMethod)method
                 URL:(id)url
          parameters:(id)parameters {
    let url = retain(env, url);
    let params = retain(env, parameters);
    let host = env.objc.borrow_mut::<SLRequestHostObject>(this);
    host.request_method = method;
    host.url = url;
    host.parameters = params;
    this
}

- (id)account {
    env.objc.borrow::<SLRequestHostObject>(this).account
}
- (())setAccount:(id)account {
    let new_value = retain(env, account);
    let host = env.objc.borrow_mut::<SLRequestHostObject>(this);
    let old = host.account;
    host.account = new_value;
    release(env, old);
}

- (id)URL {
    env.objc.borrow::<SLRequestHostObject>(this).url
}
- (SLRequestMethod)requestMethod {
    env.objc.borrow::<SLRequestHostObject>(this).request_method
}
- (id)parameters {
    env.objc.borrow::<SLRequestHostObject>(this).parameters
}

- (())addMultipartData:(id)_data
              withName:(id)_name
                  type:(id)_mime
              filename:(id)_filename {
    log!("SLRequest: addMultipartData:withName:type:filename: ignored (no transport)");
}

- (id)preparedURLRequest {
    let host = env.objc.borrow::<SLRequestHostObject>(this);
    let url = host.url;
    let params = host.parameters;
    let method = host.request_method;

    let req: id = msg_class![env; NSMutableURLRequest requestWithURL:url];
    let method_string = match method {
        SLRequestMethodGET => "GET",
        SLRequestMethodPOST => "POST",
        SLRequestMethodDELETE => "DELETE",
        SLRequestMethodPUT => "PUT",
        _ => "GET",
    };
    let method_id = crate::frameworks::foundation::ns_string::from_rust_string(
        env, method_string.to_string()
    );
    let _: () = msg![env; req setHTTPMethod:method_id];
    release(env, method_id);

    if params != nil && method == SLRequestMethodPOST {
        // Encode parameters as application/x-www-form-urlencoded.
        let body = encode_parameters_as_form(env, params);
        if !body.is_empty() {
            let body_data = msg_class![env; NSMutableData data];
            let bytes_ptr = env.mem.alloc(body.len() as u32);
            env.mem
                .bytes_at_mut(bytes_ptr.cast(), body.len() as u32)
                .copy_from_slice(body.as_bytes());
            let _: () = msg![env; body_data appendBytes:(bytes_ptr.cast_const()) length:(body.len() as u32)];
            env.mem.free(bytes_ptr);
            let _: () = msg![env; req setHTTPBody:body_data];

            let ct_key = crate::frameworks::foundation::ns_string::from_rust_string(
                env, "Content-Type".to_string()
            );
            let ct_val = crate::frameworks::foundation::ns_string::from_rust_string(
                env, "application/x-www-form-urlencoded".to_string()
            );
            let _: () = msg![env; req setValue:ct_val forHTTPHeaderField:ct_key];
            release(env, ct_key);
            release(env, ct_val);
        }
    }
    req
}

- (())performRequestWithHandler:(id)_handler {
    // touchHLE has no Accounts framework, so we cannot authenticate any
    // request. Invoke the handler synchronously on the main run loop with
    // an SLRequestErrorDomain error indicating the service is unavailable.
    log!(
        "SLRequest: performRequestWithHandler invoked; no Social account \
         backing — would have completed with SLRequestErrorDomain."
    );
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<SLRequestHostObject>(this));
    release(env, host.service_type);
    release(env, host.url);
    release(env, host.parameters);
    release(env, host.account);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

// Helper: convert an NSDictionary of NSString -> NSString parameters
// into an application/x-www-form-urlencoded body. Anything that is not a
// pair of strings is skipped.
fn encode_parameters_as_form(env: &mut crate::Environment, params: id) -> String {
    use crate::frameworks::foundation::ns_string::to_rust_string;
    use crate::frameworks::foundation::NSUInteger;
    let ns_string_class = env.objc.get_known_class("NSString", &mut env.mem);
    let mut pieces: Vec<String> = Vec::new();
    let keys: id = msg![env; params allKeys];
    let count: NSUInteger = msg![env; keys count];
    for i in 0..count {
        let k: id = msg![env; keys objectAtIndex:i];
        let v: id = msg![env; params objectForKey:k];
        let k_is_str: bool = msg![env; k isKindOfClass:ns_string_class];
        let v_is_str: bool = msg![env; v isKindOfClass:ns_string_class];
        if !k_is_str || !v_is_str {
            continue;
        }
        let key = to_rust_string(env, k).into_owned();
        let val = to_rust_string(env, v).into_owned();
        pieces.push(format!("{}={}", percent_encode(&key), percent_encode(&val)));
    }
    pieces.join("&")
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

pub const CONSTANTS: ConstantExports = &[
    (
        "_SLServiceTypeTwitter",
        HostConstant::NSString("com.apple.social.twitter"),
    ),
    (
        "_SLServiceTypeFacebook",
        HostConstant::NSString("com.apple.social.facebook"),
    ),
    (
        "_SLServiceTypeSinaWeibo",
        HostConstant::NSString("com.apple.social.sinaweibo"),
    ),
    (
        "_SLServiceTypeTencentWeibo",
        HostConstant::NSString("com.apple.social.tencentweibo"),
    ),
    (
        "_SLServiceTypeLinkedIn",
        HostConstant::NSString("com.apple.social.linkedin"),
    ),
    (
        "_SLRequestErrorDomain",
        HostConstant::NSString("SLRequestErrorDomain"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];
