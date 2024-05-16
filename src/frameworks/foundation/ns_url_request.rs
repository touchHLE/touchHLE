/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLRequest and NSMutableURLRequest`.

use super::{NSTimeInterval, NSUInteger};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::msg;
use crate::objc::{id, nil, objc_classes, ClassExports};

type NSURLRequestCachePolicy = NSUInteger;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLRequest: NSObject

+ (id)requestWithURL:(id)url
         cachePolicy:(NSURLRequestCachePolicy)cache_policy
     timeoutInterval:(NSTimeInterval)timeout_interval {
    if url == nil {
        return nil;
    }
    let url_str: id = msg![env; url path];
    log!(
        "TODO: [NSURLRequest requestWithURL:{} cachePolicy:{} timeoutInterval:{}]",
        to_rust_string(env, url_str),
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
