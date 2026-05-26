/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWebView`.

use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::msg;
use crate::objc::{id, nil, objc_classes, ClassExports};
use std::borrow::Cow;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWebView: UIView

// NSCoding implementation
- (id)initWithCoder:(id)_coder {
    todo!()
}

- (())setScalesPageToFit:(bool)_scales {
    // TODO
}
- (())setDelegate:(id)_delegate {
    // TODO
}
- (())loadRequest:(id)request { // NSURLRequest*
    let url_string = if request != nil {
        let url = msg![env; request URL];
        let url_desc = msg![env; url description];
        to_rust_string(env, url_desc)
    } else {
        Cow::default()
    };
    log!("TODO: [(UIWebView*) {:?} loadRequest:{:?} ({})]", this, request, url_string);
}

- (())loadData:(id)_data // NSData*
       MIMEType:(id)mime_type // NSString*
textEncodingName:(id)_encoding // NSString*
         baseURL:(id)_base_url { // NSURL*
    let mime = if mime_type != nil {
        to_rust_string(env, mime_type)
    } else {
        Cow::default()
    };
    let encoding = if _encoding != nil {
        to_rust_string(env, _encoding)
    } else {
        Cow::default()
    };
    let base_url_string = if _base_url != nil {
        let desc = msg![env; _base_url description];
        to_rust_string(env, desc)
    } else {
        Cow::default()
    };
    let data_len: u32 = if _data != nil {
        msg![env; _data length]
    } else {
        0
    };

    log!(
        "TODO: [(UIWebView*) {:?} loadData:{:?} ({} bytes) MIMEType:{:?} ({}) textEncodingName:{:?} ({}) baseURL:{:?} ({})]",
        this, _data, data_len, mime_type, mime, _encoding, encoding, _base_url, base_url_string
    );
}

@end

};
