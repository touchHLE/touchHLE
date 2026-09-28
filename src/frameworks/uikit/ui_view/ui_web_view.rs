/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWebView`.

use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::objc::{id, nil, objc_classes, ClassExports};
use crate::{msg, msg_super};
use std::borrow::Cow;
use std::process::Command;

fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    };

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
        return Err("external browser launching is unsupported on this platform".to_string());
    }

    let status = command
        .status()
        .map_err(|error| format!("failed to launch browser: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("browser launcher exited with status {status}"))
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWebView: UIView

// NSCoding implementation
- (id)initWithCoder:(id)_coder {
    msg_super![env; this initWithCoder:_coder]
}

- (())setScalesPageToFit:(bool)_scales {
    // TODO
}
- (())setDelegate:(id)_delegate {
    // TODO
}
- (())loadRequest:(id)request { // NSURLRequest*
    if request == nil {
        log!(
            "[(UIWebView*) {:?} loadRequest:nil] Hiding WebView and ignoring empty request.",
            this
        );
        () = msg![env; this setHidden:true];
        let container: id = msg![env; this superview];
        if container != nil {
            // Facebook's login NIB wraps the WebView in a standalone root
            // UIView. Hide that container too so it cannot become a blank
            // overlay when the NIB is attached after the request.
            () = msg![env; container setHidden:true];
        }
        return;
    }

    let url = msg![env; request URL];
    if url == nil {
        log!(
            "[(UIWebView*) {:?} loadRequest:{:?}] Hiding WebView and ignoring request without URL.",
            this,
            request
        );
        () = msg![env; this setHidden:true];
        return;
    }

    let url_desc = msg![env; url description];
    let url_string: Cow<'_, str> = to_rust_string(env, url_desc);
    if url_string.is_empty() {
        log!(
            "[(UIWebView*) {:?} loadRequest:{:?}] Hiding WebView and ignoring empty URL.",
            this,
            request
        );
        () = msg![env; this setHidden:true];
        return;
    }

    log!(
        "[(UIWebView*) {:?} loadRequest:{:?} ({})] Opening external browser.",
        this,
        request,
        url_string
    );
    if let Err(error) = open_external_url(&url_string) {
        log!("Warning: could not open UIWebView URL {:?}: {}", url_string, error);
    }
}

@end

};
