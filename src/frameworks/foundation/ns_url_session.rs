/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSURLSession`, `NSURLSessionConfiguration` and `NSURLSessionTask`
//! (iOS 7+ networking).
//!
//! This is a stub implementation that does not perform real networking, in
//! keeping with the existing `NSURLConnection` stub (see `ns_url_connection`).
//! touchHLE has no live network stack, so requests are completed with the
//! standard `NSURLErrorNotConnectedToInternet` (-1009) error in the
//! `NSURLErrorDomain`, exactly as Apple documents for the offline case.
//!
//! The shape of the API faithfully follows Apple's reference:
//! - `+[NSURLSessionConfiguration defaultSessionConfiguration]` /
//!   `ephemeralSessionConfiguration` /
//!   `backgroundSessionConfigurationWithIdentifier:`
//! - `+[NSURLSession sessionWithConfiguration:]`,
//!   `+[NSURLSession sessionWithConfiguration:delegate:delegateQueue:]`,
//!   `+[NSURLSession sharedSession]`
//! - Task creation: `dataTaskWithRequest:`, `dataTaskWithURL:` and their
//!   `completionHandler:` variants (plus download/upload spellings).
//! - `NSURLSessionTask`: `resume`, `suspend`, `cancel`, and the read-only
//!   `state`, `taskIdentifier`, `originalRequest`, `currentRequest`, `error`.
//!
//! Behaviour on `resume`:
//! - If the task was created with a completion handler, it is invoked with
//!   `(nil data, nil response, error)` — matching Apple's contract for
//!   failures.
//! - Otherwise the session delegate is notified via
//!   `URLSession:task:didCompleteWithError:` (if implemented), matching
//!   Apple's documented delegate flow.
//!
//! Returning an error (rather than fake success) is deliberate and matches
//! the `NSURLConnection` stub's rationale: faking `200 OK` with empty data
//! crashes apps that parse protocol-specific fields from the body.

use crate::abi::{CallFromHost, GuestFunction};
use crate::mem::{ConstVoidPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports,
    HostObject,
};

// NSError domain / code used when reporting "no network in emulator".
const NS_URL_ERROR_DOMAIN: &str = "NSURLErrorDomain";
const NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET: i32 = -1009;

// NSURLSessionTaskState values (Apple's enum).
const NS_URL_SESSION_TASK_STATE_RUNNING: i64 = 0;
const NS_URL_SESSION_TASK_STATE_SUSPENDED: i64 = 1;
const NS_URL_SESSION_TASK_STATE_CANCELING: i64 = 2;
const NS_URL_SESSION_TASK_STATE_COMPLETED: i64 = 3;

/// Apple Block ABI: word offset 3 (== byte offset 12) of a block struct holds
/// its `invoke` function pointer.
/// <https://clang.llvm.org/docs/Block-ABI-Apple.html>
const BLOCK_INVOKE_WORD_OFFSET: u32 = 3;

#[derive(Default)]
pub struct State {
    /// Monotonic counter used to hand out unique task identifiers, mirroring a
    /// real `NSURLSession`. A process-global counter is a faithful-enough
    /// approximation that keeps identifiers unique.
    next_task_identifier: u64,
}
impl State {
    fn get(env: &mut crate::Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_url_session
    }
}

// ---------------------------------------------------------------------------
// Host objects.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct NSURLSessionConfigurationHostObject {
    /// `NSString *` identifier for background configurations (else nil).
    identifier: id,
    allows_cellular_access: bool,
    http_additional_headers: id,
    timeout_interval_for_request: f64,
    timeout_interval_for_resource: f64,
}
impl HostObject for NSURLSessionConfigurationHostObject {}

#[derive(Default)]
struct NSURLSessionHostObject {
    configuration: id,
    delegate: id,
    delegate_queue: id,
}
impl HostObject for NSURLSessionHostObject {}

#[derive(Default)]
struct NSURLSessionTaskHostObject {
    /// The owning session (not retained — the session owns its tasks; this
    /// mirrors real NSURLSession ownership and avoids a retain cycle).
    session: id,
    /// `NSURLRequest *` — retained.
    original_request: id,
    /// The `void (^)(NSData *, NSURLResponse *, NSError *)` completion-handler
    /// block, `-copy`'d to the heap (so it survives until `resume`), or nil.
    completion_handler: id,
    state: i64,
    task_identifier: u64,
    /// Set once the task completes, so the `error` getter can report it.
    error: id,
}
impl HostObject for NSURLSessionTaskHostObject {}

// ---------------------------------------------------------------------------
// Helper — build an autoreleased NSError for "not connected to internet".
// ---------------------------------------------------------------------------
fn make_network_error(env: &mut crate::Environment) -> id {
    use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str};

    let domain = from_rust_string(env, NS_URL_ERROR_DOMAIN.to_string());
    autorelease(env, domain);

    let desc_key = get_static_str(env, "NSLocalizedDescription");
    let desc_val = from_rust_string(
        env,
        "The Internet connection appears to be offline. \
         (touchHLE: networking not supported)"
            .to_string(),
    );
    autorelease(env, desc_val);

    let user_info: id = msg_class![env; NSMutableDictionary new];
    autorelease(env, user_info);
    () = msg![env; user_info setObject:desc_val forKey:desc_key];

    let error: id = msg_class![env; NSError alloc];
    let error: id = msg![env;
        error initWithDomain:domain
                        code:NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET
                    userInfo:user_info];
    autorelease(env, error);
    error
}

/// Invoke a `void (^)(NSData *, NSURLResponse *, NSError *)` completion block.
/// Returns silently if `block` is nil or its invoke pointer is zero.
fn invoke_completion_handler(
    env: &mut crate::Environment,
    block: id,
    data: id,
    response: id,
    error: id,
) {
    if block == nil {
        return;
    }
    let block_ptr: crate::mem::MutPtr<u32> = Ptr::from_bits(block.to_bits());
    let invoke_addr: u32 = env.mem.read(block_ptr + BLOCK_INVOKE_WORD_OFFSET);
    if invoke_addr == 0 {
        log!(
            "Warning: NSURLSession completion block {:?} has NULL invoke \
             pointer; not calling.",
            block
        );
        return;
    }
    let invoke = GuestFunction::from_addr_with_thumb_bit(invoke_addr);
    let block_arg: ConstVoidPtr = Ptr::from_bits(block.to_bits()).cast_const();
    <GuestFunction as CallFromHost<(), (ConstVoidPtr, id, id, id)>>::call_from_host(
        &invoke,
        env,
        (block_arg, data, response, error),
    );
}

/// Deliver completion to a task. Marks the task completed, stores the error,
/// then either invokes its completion handler with `(nil, nil, error)` or
/// notifies the session delegate via `URLSession:task:didCompleteWithError:`.
fn deliver_task_failure(env: &mut crate::Environment, task: id) {
    // Build and retain the error we will store on the task for `-error`.
    let stored_error = make_network_error(env);
    retain(env, stored_error);

    let (handler, session, previous_error) = {
        let host = env.objc.borrow_mut::<NSURLSessionTaskHostObject>(task);
        host.state = NS_URL_SESSION_TASK_STATE_COMPLETED;
        let previous_error = host.error;
        host.error = stored_error;
        (host.completion_handler, host.session, previous_error)
    };

    // Release any error from a previous completion now that the borrow ended.
    if previous_error != nil {
        release(env, previous_error);
    }

    if handler != nil {
        let err = make_network_error(env);
        invoke_completion_handler(env, handler, nil, nil, err);
        return;
    }

    // No completion handler: notify the session delegate if present.
    let delegate: id = if session != nil {
        env.objc.borrow::<NSURLSessionHostObject>(session).delegate
    } else {
        nil
    };
    if delegate != nil {
        let err = make_network_error(env);
        () = msg![env; delegate URLSession:session task:task didCompleteWithError:err];
    }
}

/// Shared task-creation logic. Allocates an `NSURLSessionDataTask`, wires it to
/// its session and request, `-copy`s the completion-handler block to the heap
/// (so it survives until `resume`), and assigns a unique identifier.
fn make_task(env: &mut crate::Environment, session: id, request: id, handler: id) -> id {
    let task: id = msg_class![env; NSURLSessionDataTask alloc];
    let task: id = msg![env; task init];

    retain(env, request);

    // Apple's contract: blocks captured by the framework must be copied to the
    // heap. `-copy` on a stack block produces a heap block; on a heap block
    // it's a retain.
    let copied_handler: id = if handler == nil {
        nil
    } else {
        msg![env; handler copy]
    };

    let identifier = {
        let state = State::get(env);
        let id_value = state.next_task_identifier;
        state.next_task_identifier += 1;
        id_value
    };

    {
        let host = env.objc.borrow_mut::<NSURLSessionTaskHostObject>(task);
        host.session = session;
        host.original_request = request;
        host.completion_handler = copied_handler;
        host.state = NS_URL_SESSION_TASK_STATE_SUSPENDED;
        host.task_identifier = identifier;
        host.error = nil;
    }

    autorelease(env, task)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// ===========================================================================
// NSURLSessionConfiguration
// ===========================================================================
@implementation NSURLSessionConfiguration: NSObject

+ (id)allocWithZone:(crate::objc::NSZonePtr)_zone {
    let host = Box::<NSURLSessionConfigurationHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)defaultSessionConfiguration {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    autorelease(env, new)
}

+ (id)ephemeralSessionConfiguration {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    autorelease(env, new)
}

+ (id)backgroundSessionConfigurationWithIdentifier:(id)identifier {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    retain(env, identifier);
    env.objc
        .borrow_mut::<NSURLSessionConfigurationHostObject>(new)
        .identifier = identifier;
    autorelease(env, new)
}

// Deprecated spelling kept by some apps.
+ (id)backgroundSessionConfiguration:(id)identifier {
    msg![env; this backgroundSessionConfigurationWithIdentifier:identifier]
}

- (id)init {
    let this: id = msg_super![env; this init];
    {
        let host = env
            .objc
            .borrow_mut::<NSURLSessionConfigurationHostObject>(this);
        host.allows_cellular_access = true;
        // Apple defaults: 60s request timeout, 7 days resource timeout.
        host.timeout_interval_for_request = 60.0;
        host.timeout_interval_for_resource = 604800.0;
    }
    this
}

- (id)identifier {
    env.objc
        .borrow::<NSURLSessionConfigurationHostObject>(this)
        .identifier
}

- (bool)allowsCellularAccess {
    env.objc
        .borrow::<NSURLSessionConfigurationHostObject>(this)
        .allows_cellular_access
}
- (())setAllowsCellularAccess:(bool)value {
    env.objc
        .borrow_mut::<NSURLSessionConfigurationHostObject>(this)
        .allows_cellular_access = value;
}

- (id)HTTPAdditionalHeaders {
    env.objc
        .borrow::<NSURLSessionConfigurationHostObject>(this)
        .http_additional_headers
}
- (())setHTTPAdditionalHeaders:(id)headers {
    retain(env, headers);
    let old = env
        .objc
        .borrow::<NSURLSessionConfigurationHostObject>(this)
        .http_additional_headers;
    env.objc
        .borrow_mut::<NSURLSessionConfigurationHostObject>(this)
        .http_additional_headers = headers;
    release(env, old);
}

- (f64)timeoutIntervalForRequest {
    env.objc
        .borrow::<NSURLSessionConfigurationHostObject>(this)
        .timeout_interval_for_request
}
- (())setTimeoutIntervalForRequest:(f64)value {
    env.objc
        .borrow_mut::<NSURLSessionConfigurationHostObject>(this)
        .timeout_interval_for_request = value;
}

- (f64)timeoutIntervalForResource {
    env.objc
        .borrow::<NSURLSessionConfigurationHostObject>(this)
        .timeout_interval_for_resource
}
- (())setTimeoutIntervalForResource:(f64)value {
    env.objc
        .borrow_mut::<NSURLSessionConfigurationHostObject>(this)
        .timeout_interval_for_resource = value;
}

- (())dealloc {
    let &NSURLSessionConfigurationHostObject {
        identifier,
        http_additional_headers,
        ..
    } = env
        .objc
        .borrow::<NSURLSessionConfigurationHostObject>(this);
    release(env, identifier);
    release(env, http_additional_headers);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

// ===========================================================================
// NSURLSession
// ===========================================================================
@implementation NSURLSession: NSObject

+ (id)allocWithZone:(crate::objc::NSZonePtr)_zone {
    let host = Box::<NSURLSessionHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)sharedSession {
    let config: id = msg_class![env; NSURLSessionConfiguration defaultSessionConfiguration];
    msg![env; this sessionWithConfiguration:config]
}

+ (id)sessionWithConfiguration:(id)configuration {
    msg![env; this sessionWithConfiguration:configuration delegate:nil delegateQueue:nil]
}

+ (id)sessionWithConfiguration:(id)configuration
                      delegate:(id)delegate
                 delegateQueue:(id)queue {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];

    retain(env, configuration);
    retain(env, delegate);
    retain(env, queue);
    {
        let host = env.objc.borrow_mut::<NSURLSessionHostObject>(new);
        host.configuration = configuration;
        host.delegate = delegate;
        host.delegate_queue = queue;
    }
    autorelease(env, new)
}

- (id)configuration {
    env.objc.borrow::<NSURLSessionHostObject>(this).configuration
}
- (id)delegate {
    env.objc.borrow::<NSURLSessionHostObject>(this).delegate
}
- (id)delegateQueue {
    env.objc.borrow::<NSURLSessionHostObject>(this).delegate_queue
}

// MARK: - Task creation

- (id)dataTaskWithRequest:(id)request {
    make_task(env, this, request, nil)
}

- (id)dataTaskWithRequest:(id)request
        completionHandler:(id)handler {
    make_task(env, this, request, handler)
}

- (id)dataTaskWithURL:(id)url {
    let request: id = msg_class![env; NSURLRequest requestWithURL:url];
    make_task(env, this, request, nil)
}

- (id)dataTaskWithURL:(id)url
    completionHandler:(id)handler {
    let request: id = msg_class![env; NSURLRequest requestWithURL:url];
    make_task(env, this, request, handler)
}

// downloadTask*/uploadTask* share the same offline behaviour; we vend the
// same task object type so apps get a graceful failure either way.
- (id)downloadTaskWithRequest:(id)request {
    make_task(env, this, request, nil)
}
- (id)downloadTaskWithRequest:(id)request
            completionHandler:(id)handler {
    make_task(env, this, request, handler)
}
- (id)downloadTaskWithURL:(id)url {
    let request: id = msg_class![env; NSURLRequest requestWithURL:url];
    make_task(env, this, request, nil)
}
- (id)downloadTaskWithURL:(id)url
        completionHandler:(id)handler {
    let request: id = msg_class![env; NSURLRequest requestWithURL:url];
    make_task(env, this, request, handler)
}

// MARK: - Lifecycle

- (())finishTasksAndInvalidate {
    // No background tasks to drain in the stub.
}
- (())invalidateAndCancel {
    // Drop strong refs to delegate/queue as a real invalidated session does.
    let &NSURLSessionHostObject { delegate, delegate_queue, .. } =
        env.objc.borrow::<NSURLSessionHostObject>(this);
    release(env, delegate);
    release(env, delegate_queue);
    let host = env.objc.borrow_mut::<NSURLSessionHostObject>(this);
    host.delegate = nil;
    host.delegate_queue = nil;
}

- (())dealloc {
    let &NSURLSessionHostObject {
        configuration,
        delegate,
        delegate_queue,
    } = env.objc.borrow::<NSURLSessionHostObject>(this);
    release(env, configuration);
    release(env, delegate);
    release(env, delegate_queue);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

// ===========================================================================
// NSURLSessionTask (data/download/upload tasks all use this host class)
// ===========================================================================
@implementation NSURLSessionTask: NSObject

+ (id)allocWithZone:(crate::objc::NSZonePtr)_zone {
    let host = Box::<NSURLSessionTaskHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (())resume {
    let state = env
        .objc
        .borrow::<NSURLSessionTaskHostObject>(this)
        .state;
    if state == NS_URL_SESSION_TASK_STATE_COMPLETED
        || state == NS_URL_SESSION_TASK_STATE_CANCELING
    {
        return;
    }
    env.objc
        .borrow_mut::<NSURLSessionTaskHostObject>(this)
        .state = NS_URL_SESSION_TASK_STATE_RUNNING;
    log!(
        "NSURLSessionTask resume: delivering NSURLErrorNotConnectedToInternet \
         (touchHLE has no network)"
    );
    deliver_task_failure(env, this);
}

- (())suspend {
    let host = env.objc.borrow_mut::<NSURLSessionTaskHostObject>(this);
    if host.state == NS_URL_SESSION_TASK_STATE_RUNNING {
        host.state = NS_URL_SESSION_TASK_STATE_SUSPENDED;
    }
}

- (())cancel {
    let host = env.objc.borrow_mut::<NSURLSessionTaskHostObject>(this);
    if host.state != NS_URL_SESSION_TASK_STATE_COMPLETED {
        host.state = NS_URL_SESSION_TASK_STATE_CANCELING;
    }
}

- (i64)state {
    env.objc.borrow::<NSURLSessionTaskHostObject>(this).state
}

- (u64)taskIdentifier {
    env.objc
        .borrow::<NSURLSessionTaskHostObject>(this)
        .task_identifier
}

- (id)originalRequest {
    env.objc
        .borrow::<NSURLSessionTaskHostObject>(this)
        .original_request
}
- (id)currentRequest {
    env.objc
        .borrow::<NSURLSessionTaskHostObject>(this)
        .original_request
}
- (id)response {
    nil
}
- (id)error {
    env.objc.borrow::<NSURLSessionTaskHostObject>(this).error
}

- (())dealloc {
    let &NSURLSessionTaskHostObject {
        original_request,
        completion_handler,
        error,
        ..
    } = env.objc.borrow::<NSURLSessionTaskHostObject>(this);
    release(env, original_request);
    release(env, completion_handler);
    release(env, error);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

// Subclass spellings used by apps that down-cast; they all behave identically
// to NSURLSessionTask in this offline stub.
@implementation NSURLSessionDataTask: NSURLSessionTask
@end

@implementation NSURLSessionDownloadTask: NSURLSessionTask
@end

@implementation NSURLSessionUploadTask: NSURLSessionTask
@end

};
