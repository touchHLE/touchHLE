/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLRequest and NSMutableURLRequest`.

use touchhle_macros::validate_class_exports;

use super::{NSTimeInterval, NSUInteger};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::msg;
use crate::objc::{id, nil, objc_classes, ClassExports};

type NSURLRequestCachePolicy = NSUInteger;
const NSURLRequestUseProtocolCachePolicy: NSURLRequestCachePolicy = 0;

#[validate_class_exports]
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLRequest: NSObject

+ (id)requestWithURL:(id)url {
    msg![env; this requestWithURL:url
                      cachePolicy:NSURLRequestUseProtocolCachePolicy
                  timeoutInterval:60.0]
}

+ (id)requestWithURL:(id)url
         cachePolicy:(NSURLRequestCachePolicy)cache_policy
     timeoutInterval:(NSTimeInterval)timeout_interval {
    if url == nil {
        return nil;
    }
    let url_desc: id = msg![env; url description];
    log!(
        "TODO: [NSURLRequest requestWithURL:{} cachePolicy:{} timeoutInterval:{}]",
        to_rust_string(env, url_desc),
        cache_policy,
        timeout_interval,
    );
    nil
}

@end

@implementation NSMutableURLRequest: NSURLRequest
//TODO
@end

};
