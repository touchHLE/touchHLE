/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWebView`.

use crate::frameworks::core_graphics::{cg_image, CGRect};
use crate::frameworks::foundation::ns_string::{self, to_rust_string};
use crate::frameworks::foundation::NSUInteger;
use crate::frameworks::uikit::ui_view::UIViewHostObject;
use crate::image::Image;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};
use crate::Environment;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

// UIWebViewNavigationType constants
pub type UIWebViewNavigationType = i32;
pub const UIWebViewNavigationTypeLinkClicked: UIWebViewNavigationType = 0;
pub const UIWebViewNavigationTypeFormSubmitted: UIWebViewNavigationType = 1;
pub const UIWebViewNavigationTypeBackForward: UIWebViewNavigationType = 2;
pub const UIWebViewNavigationTypeReload: UIWebViewNavigationType = 3;
pub const UIWebViewNavigationTypeFormResubmitted: UIWebViewNavigationType = 4;
pub const UIWebViewNavigationTypeOther: UIWebViewNavigationType = 5;

// UIDataDetectorTypes bitmask
pub type UIDataDetectorTypes = NSUInteger;
pub const UIDataDetectorTypePhoneNumber: UIDataDetectorTypes = 1 << 0;
pub const UIDataDetectorTypeLink: UIDataDetectorTypes = 1 << 1;
pub const UIDataDetectorTypeAddress: UIDataDetectorTypes = 1 << 2;
pub const UIDataDetectorTypeCalendarEvent: UIDataDetectorTypes = 1 << 3;
pub const UIDataDetectorTypeNone: UIDataDetectorTypes = 0;
pub const UIDataDetectorTypeAll: UIDataDetectorTypes = u32::MAX as UIDataDetectorTypes;

#[derive(Default)]
struct UIWebViewHostObject {
    superclass: UIViewHostObject,
    /// UIWebViewDelegate — weak reference (no retain per Apple docs)
    delegate: id,
    scales_page_to_fit: bool,
    detects_phone_numbers: bool,
    data_detector_types: UIDataDetectorTypes,
    allows_inline_media_playback: bool,
    media_playback_requires_user_action: bool,
    media_playback_allows_air_play: bool,
    suppress_incremental_rendering: bool,
    keyboard_display_requires_user_action: bool,
    pagination_mode: i32,
    pagination_breaking_mode: i32,
    page_length: f64,
    gap_between_pages: f64,
    /// NSString* — last URL string passed to loadRequest:
    current_url: id,

    loading: bool,
    /// Simple back/forward stack — NSString* items.
    back_stack: Vec<id>,
    forward_stack: Vec<id>,
}
impl_HostObject_with_superclass!(UIWebViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWebView: UIView

// =========================================================================
// MARK: - Allocation
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIWebViewHostObject {
        superclass: UIViewHostObject::default(),
        delegate: nil,
        scales_page_to_fit: false,
        detects_phone_numbers: true,
        data_detector_types: UIDataDetectorTypePhoneNumber,
        allows_inline_media_playback: false,
        media_playback_requires_user_action: true,
        media_playback_allows_air_play: true,
        suppress_incremental_rendering: false,
        keyboard_display_requires_user_action: true,

        pagination_mode: 0,
        pagination_breaking_mode: 0,
        page_length: 0.0,
        gap_between_pages: 0.0,
        current_url: nil,
        loading: false,
        back_stack: Vec::new(),
        forward_stack: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// =========================================================================
// MARK: - Initializers
// =========================================================================

- (id)init {
    this
}

- (id)initWithFrame:(CGRect)_frame {
    this
}

- (id)initWithCoder:(id)_coder {
    this
}

// =========================================================================
// MARK: - Dealloc
// =========================================================================

- (())dealloc {
    let host = env.objc.borrow::<UIWebViewHostObject>(this);
    let (current_url, back_stack, forward_stack) = (
        host.current_url,
        host.back_stack.clone(),
        host.forward_stack.clone(),
    );
    release(env, current_url);
    for url in back_stack    { release(env, url); }
    for url in forward_stack { release(env, url); }
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// MARK: - Delegate
// =========================================================================

- (id)delegate {
    env.objc.borrow::<UIWebViewHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    // Weak reference — do NOT retain.
    env.objc.borrow_mut::<UIWebViewHostObject>(this).delegate = delegate;
}

// =========================================================================
// MARK: - Loading
// =========================================================================

- (())loadRequest:(id)request { // NSURLRequest*
    let url_string: String = if request != nil {
        let url: id = msg![env; request URL];
        let url_desc: id = msg![env; url description];
        if url_desc != nil { to_rust_string(env, url_desc).into_owned() } else { String::new() }
    } else {
        String::new()
    };
    log!("UIWebView loadRequest: {}", url_string);

    let frame: CGRect = msg![env; this frame];
    render_url_to_layer(env, this, &url_string, frame);

    // Push current URL onto back stack before navigating.
    let old_url = env.objc.borrow::<UIWebViewHostObject>(this).current_url;
    if old_url != nil {
        retain(env, old_url);
        env.objc.borrow_mut::<UIWebViewHostObject>(this).back_stack.push(old_url);
        // Clear forward stack on new navigation.
        let fwd: Vec<id> = std::mem::take(
            &mut env.objc.borrow_mut::<UIWebViewHostObject>(this).forward_stack
        );
        for u in fwd { release(env, u); }
    }
    release(env, old_url);
    let ns_url = ns_string::from_rust_string(env, url_string.clone());
    {
        let host = env.objc.borrow_mut::<UIWebViewHostObject>(this);
        host.current_url = ns_url;
        host.loading = true;
    }

    // Fire webViewDidStartLoad: delegate callback. Use `register_host_selector`
    // rather than `lookup_selector(...).unwrap()` so that we don't panic if the
    // app never references this delegate selector (e.g. Minecraft PE 0.8.0
    // doesn't implement the UIWebViewDelegate protocol callbacks).
    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webViewDidStartLoad:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate webViewDidStartLoad:this];
        }
    }

    // Since we don't actually load anything, immediately fire didFinishLoad:.
    env.objc.borrow_mut::<UIWebViewHostObject>(this).loading = false;
    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webViewDidFinishLoad:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate webViewDidFinishLoad:this];
        }
    }
}

- (())loadHTMLString:(id)html     // NSString*
            baseURL:(id)_base_url { // NSURL*
    let len = if html != nil {
        to_rust_string(env, html).len()
    } else { 0 };
    log_dbg!("UIWebView loadHTMLString: ({} chars)", len);
    env.objc.borrow_mut::<UIWebViewHostObject>(this).loading = false;

    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webViewDidFinishLoad:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate webViewDidFinishLoad:this];
        }
    }
}

- (())loadData:(id)_data           // NSData*
      MIMEType:(id)mime            // NSString*
      textEncodingName:(id)_enc    // NSString*
       baseURL:(id)_base_url {     // NSURL*
    let mime_str = if mime != nil { to_rust_string(env, mime).into_owned() } else { "(null)".into() };
    log_dbg!("UIWebView loadData:MIMEType:{}", mime_str);
    env.objc.borrow_mut::<UIWebViewHostObject>(this).loading = false;

    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webViewDidFinishLoad:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate webViewDidFinishLoad:this];
        }
    }
}

// =========================================================================
// MARK: - Subview management
// =========================================================================

- (())insertSubview:(id)view aboveSubview:(id)sibling {
    if view == nil { return; }

    // If sibling is nil or not in our view hierarchy, just add at the top.
    if sibling == nil {
        let _: () = msg![env; this addSubview:view];
        return;
    }

    // Delegate to UIView's insertSubview:aboveSubview: on our own view.
    let self_view: id = msg![env; this view];
    if self_view != nil {
        let _: () = msg![env; self_view insertSubview:view aboveSubview:sibling];
    } else {
        // Fallback — just add it.
        let _: () = msg![env; this addSubview:view];
    }
}

- (())reload {
    log_dbg!("UIWebView reload");
    let current = env.objc.borrow::<UIWebViewHostObject>(this).current_url;
    if current == nil { return; }
    retain(env, current);

    // Вынесено в отдельную переменную для предотвращения ошибки E0283
    let url: id = msg_class![env; NSURL URLWithString:current];
    let ns_req: id = msg_class![env; NSURLRequest requestWithURL:url];

    let _: () = msg![env; this loadRequest:ns_req];
    release(env, current);
}

- (())stopLoading {
    log_dbg!("UIWebView stopLoading");
    env.objc.borrow_mut::<UIWebViewHostObject>(this).loading = false;

    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webView:didFailLoadWithError:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let error: id = nil;
            let _: () = msg![env; delegate webView:this didFailLoadWithError:error];
        }
    }
}

- (bool)isLoading {
    env.objc.borrow::<UIWebViewHostObject>(this).loading
}

// =========================================================================
// MARK: - Navigation
// =========================================================================

- (bool)canGoBack {
    !env.objc.borrow::<UIWebViewHostObject>(this).back_stack.is_empty()
}

- (bool)canGoForward {
    !env.objc.borrow::<UIWebViewHostObject>(this).forward_stack.is_empty()
}

- (())goBack {
    let back_url = {
        let host = env.objc.borrow_mut::<UIWebViewHostObject>(this);
        host.back_stack.pop()
    };
    let Some(back_url) = back_url else { return; };

    // Push current to forward stack.
    let current = env.objc.borrow::<UIWebViewHostObject>(this).current_url;
    if current != nil {
        retain(env, current);
        env.objc.borrow_mut::<UIWebViewHostObject>(this).forward_stack.push(current);
    }
    release(env, current);

    env.objc.borrow_mut::<UIWebViewHostObject>(this).current_url = back_url;
    env.objc.borrow_mut::<UIWebViewHostObject>(this).loading = false;

    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webViewDidFinishLoad:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate webViewDidFinishLoad:this];
        }
    }
}

- (())goForward {
    let fwd_url = {
        let host = env.objc.borrow_mut::<UIWebViewHostObject>(this);
        host.forward_stack.pop()
    };
    let Some(fwd_url) = fwd_url else { return; };

    let current = env.objc.borrow::<UIWebViewHostObject>(this).current_url;
    if current != nil {
        retain(env, current);
        env.objc.borrow_mut::<UIWebViewHostObject>(this).back_stack.push(current);
    }
    release(env, current);

    env.objc.borrow_mut::<UIWebViewHostObject>(this).current_url = fwd_url;
    env.objc.borrow_mut::<UIWebViewHostObject>(this).loading = false;

    let delegate = env.objc.borrow::<UIWebViewHostObject>(this).delegate;
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "webViewDidFinishLoad:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            let _: () = msg![env; delegate webViewDidFinishLoad:this];
        }
    }
}

// =========================================================================
// MARK: - JavaScript
// =========================================================================

- (id)stringByEvaluatingJavaScriptFromString:(id)script { // NSString* -> NSString*
    let script_str = if script != nil {
        to_rust_string(env, script).into_owned()
    } else { String::new() };
    log_dbg!("UIWebView stringByEvaluatingJavaScriptFromString: {:?} — returning empty string", script_str);
    // Return empty NSString rather than nil — some apps check the return value.
    let empty = ns_string::from_rust_string(env, String::new());
    crate::objc::autorelease(env, empty)
}

// =========================================================================
// MARK: - Request / URL accessors
// =========================================================================

- (id)request { // NSURLRequest*
    nil
}

// Returns the URL of the currently loaded page as an NSString*.
- (id)_currentURLString { // NSString* (private helper)
    env.objc.borrow::<UIWebViewHostObject>(this).current_url
}

// =========================================================================
// MARK: - Properties
// =========================================================================

- (bool)scalesPageToFit {
    env.objc.borrow::<UIWebViewHostObject>(this).scales_page_to_fit
}
- (())setScalesPageToFit:(bool)scales {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).scales_page_to_fit = scales;
}

- (bool)detectsPhoneNumbers {
    env.objc.borrow::<UIWebViewHostObject>(this).detects_phone_numbers
}
- (())setDetectsPhoneNumbers:(bool)value {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).detects_phone_numbers = value;
}

- (UIDataDetectorTypes)dataDetectorTypes {
    env.objc.borrow::<UIWebViewHostObject>(this).data_detector_types
}
- (())setDataDetectorTypes:(UIDataDetectorTypes)types {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).data_detector_types = types;
}

- (bool)allowsInlineMediaPlayback {
    env.objc.borrow::<UIWebViewHostObject>(this).allows_inline_media_playback
}
- (())setAllowsInlineMediaPlayback:(bool)value {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).allows_inline_media_playback = value;
}

- (bool)mediaPlaybackRequiresUserAction {
    env.objc.borrow::<UIWebViewHostObject>(this).media_playback_requires_user_action
}
- (())setMediaPlaybackRequiresUserAction:(bool)value {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).media_playback_requires_user_action = value;
}

- (bool)mediaPlaybackAllowsAirPlay {
    env.objc.borrow::<UIWebViewHostObject>(this).media_playback_allows_air_play
}
- (())setMediaPlaybackAllowsAirPlay:(bool)value {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).media_playback_allows_air_play = value;
}

- (bool)suppressesIncrementalRendering {
    env.objc.borrow::<UIWebViewHostObject>(this).suppress_incremental_rendering
}
- (())setSuppressesIncrementalRendering:(bool)value {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).suppress_incremental_rendering = value;
}

- (bool)keyboardDisplayRequiresUserAction {
    env.objc.borrow::<UIWebViewHostObject>(this).keyboard_display_requires_user_action
}
- (())setKeyboardDisplayRequiresUserAction:(bool)value {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).keyboard_display_requires_user_action = value;
}

// Pagination (iOS 7+)
- (i32)paginationMode { // UIWebPaginationMode
    env.objc.borrow::<UIWebViewHostObject>(this).pagination_mode
}
- (())setPaginationMode:(i32)mode {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).pagination_mode = mode;
}

- (i32)paginationBreakingMode { // UIWebPaginationBreakingMode
    env.objc.borrow::<UIWebViewHostObject>(this).pagination_breaking_mode
}
- (())setPaginationBreakingMode:(i32)mode {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).pagination_breaking_mode = mode;
}

- (f64)pageLength {
    env.objc.borrow::<UIWebViewHostObject>(this).page_length
}
- (())setPageLength:(f64)length {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).page_length = length;
}

- (f64)gapBetweenPages {
    env.objc.borrow::<UIWebViewHostObject>(this).gap_between_pages
}
- (())setGapBetweenPages:(f64)gap {
    env.objc.borrow_mut::<UIWebViewHostObject>(this).gap_between_pages = gap;
}

- (u32)pageCount { // NSUInteger
    // No real rendering — always 0.
    0u32
}

// =========================================================================
// MARK: - Scroll view
// =========================================================================

// Returns a stub scroll view so apps that access scrollView don't crash.
- (id)scrollView { // UIScrollView*
    // Return self as a passthrough — we don't have a real UIScrollView here.
    this
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let (loading, current_url) = {
        let h = env.objc.borrow::<UIWebViewHostObject>(this);
        (h.loading, h.current_url)
    };
    let url_str = if current_url != nil {
        to_rust_string(env, current_url).into_owned()
    } else { "(nil)".into() };
    let s = format!(
        "<UIWebView: {:?}; loading={}; url={}>",
        this, loading, url_str
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

// =========================================================================
// MARK: - Chromium/CDP bridge: render a URL into the view's layer.contents
// =========================================================================
//
// touchHLE has no HTML rendering engine. As an opportunistic fallback (see
// PR description) we shell out to the host's headless Chromium to rasterise
// the target URL into a PNG, then install that PNG as the CALayer contents
// for the UIWebView. This gives apps like Google Mobile a visible web page
// instead of a blank rectangle, at the cost of interactivity.

/// Counter used for unique temp filenames.
static SNAP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Find a headless-capable Chromium binary on the host. Returns `None` when
/// no suitable browser is available — in that case we leave the layer blank.
fn find_chromium_binary() -> Option<PathBuf> {
    // Allow env var override for advanced users / CI.
    if let Ok(path) = std::env::var("TOUCHHLE_CHROMIUM") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        "/opt/.devin/chrome/chrome/linux-137.0.7118.2/chrome-linux64/chrome",
        "/opt/.devin/playwright_browsers/chromium-1097/chrome-linux/chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome-stable",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Shell out to headless Chromium to snapshot `url` at `width x height` and
/// return the resulting PNG bytes. Returns `None` on any failure; callers
/// are expected to treat that as "leave the layer blank".
fn snapshot_url_with_chromium(url: &str, width: u32, height: u32) -> Option<Vec<u8>> {
    let Some(chrome) = find_chromium_binary() else {
        log!("UIWebView bridge: no Chromium binary found; leaving layer blank");
        return None;
    };
    let idx = SNAP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("touchhle_uiwebview_{}.png", idx));
    // Chromium refuses to run as root unless given --no-sandbox.
    let status = Command::new(&chrome)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--hide-scrollbars")
        .arg("--no-sandbox")
        .arg("--disable-dev-shm-usage")
        .arg(format!("--window-size={},{}", width, height))
        .arg(format!("--screenshot={}", tmp.display()))
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            log!("UIWebView bridge: chromium exited with {}", s);
        }
        Err(e) => {
            log!("UIWebView bridge: failed to spawn chromium: {}", e);
            return None;
        }
    }
    let bytes = std::fs::read(&tmp).ok();
    let _ = std::fs::remove_file(&tmp);
    bytes
}

/// Snapshot `url` and install the decoded PNG as the UIWebView's
/// `layer.contents` so the user sees the rendered web page.
fn render_url_to_layer(env: &mut Environment, this: id, url: &str, frame: CGRect) {
    if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
        return;
    }
    let width = (frame.size.width.max(1.0) as u32).max(1);
    let height = (frame.size.height.max(1.0) as u32).max(1);
    let Some(png) = snapshot_url_with_chromium(url, width, height) else {
        return;
    };
    let Ok(image) = Image::from_bytes(&png) else {
        log!("UIWebView bridge: could not decode PNG snapshot");
        return;
    };
    let cg_image = cg_image::from_image(env, image);
    let layer: id = msg![env; this layer];
    let _: () = msg![env; layer setContents:cg_image];
    let _: () = msg![env; this setNeedsDisplay];
    cg_image::CGImageRelease(env, cg_image);
}
