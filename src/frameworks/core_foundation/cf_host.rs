/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code, non_snake_case, non_upper_case_globals)]
//! `CFHost` — CFNetwork host resolution.
//!
//! Implements a subset of the deprecated CFHost API from
//! `<CFNetwork/CFHost.h>`:
//!
//! ```c
//! CFHostRef CFHostCreateWithName  (CFAllocatorRef, CFStringRef);
//! CFHostRef CFHostCreateWithAddress(CFAllocatorRef, CFDataRef);
//! CFHostRef CFHostCreateCopy      (CFAllocatorRef, CFHostRef);
//!
//! Boolean   CFHostStartInfoResolution (CFHostRef, CFHostInfoType,
//!                                      CFStreamError*);
//! void      CFHostCancelInfoResolution(CFHostRef, CFHostInfoType);
//! Boolean   CFHostIsInfoResolved      (CFHostRef, CFHostInfoType);
//!
//! void      CFHostScheduleWithRunLoop   (CFHostRef, CFRunLoopRef,
//!                                        CFStringRef mode);
//! void      CFHostUnscheduleFromRunLoop (CFHostRef, CFRunLoopRef,
//!                                        CFStringRef mode);
//!
//! Boolean   CFHostSetClient (CFHostRef, CFHostClientCallBack,
//!                            CFHostClientContext*);
//!
//! CFArrayRef CFHostGetAddressing   (CFHostRef, Boolean* hasBeenResolved);
//! CFArrayRef CFHostGetNames        (CFHostRef, Boolean* hasBeenResolved);
//! CFDataRef  CFHostGetReachability (CFHostRef, Boolean* hasBeenResolved);
//! CFTypeID   CFHostGetTypeID       (void);
//! ```
//!
//! DNS resolution is performed synchronously inside
//! `CFHostStartInfoResolution` on the emulator thread. When a run loop has
//! been scheduled via `CFHostScheduleWithRunLoop` (the normal asynchronous
//! flow used by real iOS apps) the registered client callout is still
//! invoked from `CFHostStartInfoResolution` after the lookup completes,
//! which is indistinguishable from an asynchronous resolution that
//! completed before the caller returned. Apple documents the callout as
//! running on the scheduled run loop once resolution finishes, and this
//! keeps the touchHLE runtime single-threaded and simple.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use crate::frameworks::core_foundation::cf_array::{CFArrayAppendValue, CFArrayCreateMutable};
use crate::frameworks::core_foundation::cf_data::{
    CFDataCreate, CFDataGetBytePtr, CFDataGetLength,
};
use crate::frameworks::core_foundation::cf_type::CFTypeID;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::ns_string;
use crate::libc::sys::socket::sockaddr;
use crate::mem::{ConstVoidPtr, GuestISize, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::objc::{nil, objc_classes, release, Class, ClassExports, HostObject};
use crate::Environment;
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};

pub type CFHostRef = CFTypeRef;

// CFHostInfoType (enum, but ABI is a uint32)
pub type CFHostInfoType = u32;
pub const kCFHostAddresses: CFHostInfoType = 0;
pub const kCFHostNames: CFHostInfoType = 1;
pub const kCFHostReachability: CFHostInfoType = 2;
const INFO_TYPE_COUNT: usize = 3;

/// `CFStreamError` — (domain, error).
///
/// Layout on 32-bit iOS: `{ CFIndex domain; SInt32 error; }` — 8 bytes.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub struct CFStreamError {
    pub domain: i32,
    pub error: i32,
}
unsafe impl SafeRead for CFStreamError {}

// Stream error domains.
const kCFStreamErrorDomainPOSIX: i32 = 1;
const kCFStreamErrorDomainMacOSStatus: i32 = 2;
const kCFStreamErrorDomainNetDB: i32 = 12;

/// `CFHostClientContext`.
///
/// ```c
/// typedef struct {
///     CFIndex                            version;
///     void*                              info;
///     CFAllocatorRetainCallBack          retain;
///     CFAllocatorReleaseCallBack         release;
///     CFAllocatorCopyDescriptionCallBack copyDescription;
/// } CFHostClientContext;
/// ```
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct CFHostClientContext {
    pub version: GuestISize,
    pub info: MutVoidPtr,
    pub retain: GuestFunction,
    pub release: GuestFunction,
    pub copy_description: GuestFunction,
}
unsafe impl SafeRead for CFHostClientContext {}

/// Entry for a run loop/mode pair the host has been scheduled on.
#[derive(Clone)]
struct ScheduleEntry {
    /// Retained CFRunLoopRef.
    run_loop: CFTypeRef,
    /// Owned Rust copy of the run loop mode string so we don't hold onto the
    /// guest NSString* indefinitely.
    mode: String,
}

#[derive(Default)]
pub struct CFHostHostObject {
    /// `Some` when created via `CFHostCreateWithName`.
    pub name: Option<String>,
    /// `Some` when created via `CFHostCreateWithAddress`.
    pub address: Option<SocketAddrV4>,
    /// Populated by a successful `CFHostStartInfoResolution(kCFHostAddresses)`.
    pub resolved_addresses: Vec<SocketAddrV4>,
    /// Populated by a successful `CFHostStartInfoResolution(kCFHostNames)`.
    pub resolved_names: Vec<String>,
    /// Resolution completion, indexed by `CFHostInfoType as usize`.
    resolved: [bool; INFO_TYPE_COUNT],
    /// Rejects concurrent calls of the same kind.
    in_progress: [bool; INFO_TYPE_COUNT],
    /// Set by `CFHostCancelInfoResolution`; checked before firing the callout.
    cancelled: [bool; INFO_TYPE_COUNT],
    /// Guest `CFHostClientCallBack`, set via `CFHostSetClient`.
    pub callout: Option<GuestFunction>,
    /// `info` pointer from the `CFHostClientContext` (after `retain`, if one
    /// was supplied by the caller).
    pub client_info: MutVoidPtr,
    pub client_retain: Option<GuestFunction>,
    pub client_release: Option<GuestFunction>,
    /// Run loops the host has been scheduled on.
    scheduled: Vec<ScheduleEntry>,
}

impl HostObject for CFHostHostObject {}

pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);

@implementation _touchHLE_CFHost: NSObject
- (())dealloc {
    // Release the client context info if the caller gave us a release
    // callback, and unretain any run loops we retained in
    // CFHostScheduleWithRunLoop.
    let (client_release, client_info, scheduled) = {
        let h = env.objc.borrow_mut::<CFHostHostObject>(this);
        let release_cb = h.client_release.take();
        let info = std::mem::take(&mut h.client_info);
        let scheduled = std::mem::take(&mut h.scheduled);
        h.callout = None;
        h.client_retain = None;
        (release_cb, info, scheduled)
    };

    if let Some(release_cb) = client_release {
        if !client_info.is_null() {
            let _: () = release_cb.call_from_host(env, (client_info,));
        }
    }

    for entry in scheduled {
        if !entry.run_loop.is_null() {
            CFRelease(env, entry.run_loop);
        }
    }

    env.objc.dealloc_object(this, &mut env.mem)
}
@end

};

// MARK: - Internal helpers

fn alloc_cfhost(
    env: &mut Environment,
    name: Option<String>,
    address: Option<SocketAddrV4>,
) -> CFHostRef {
    let class = env.objc.get_known_class("_touchHLE_CFHost", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CFHostHostObject {
            name,
            address,
            resolved_addresses: Vec::new(),
            resolved_names: Vec::new(),
            resolved: [false; INFO_TYPE_COUNT],
            in_progress: [false; INFO_TYPE_COUNT],
            cancelled: [false; INFO_TYPE_COUNT],
            callout: None,
            client_info: MutVoidPtr::null(),
            client_retain: None,
            client_release: None,
            scheduled: Vec::new(),
        }),
        &mut env.mem,
    )
}

fn write_stream_error(env: &mut Environment, error_ptr: MutPtr<CFStreamError>, err: CFStreamError) {
    if !error_ptr.is_null() {
        env.mem.write(error_ptr, err);
    }
}

// MARK: - Lifecycle

pub fn CFHostRetain(env: &mut Environment, host: CFHostRef) -> CFHostRef {
    if !host.is_null() {
        CFRetain(env, host)
    } else {
        host
    }
}

pub fn CFHostRelease(env: &mut Environment, host: CFHostRef) {
    if !host.is_null() {
        CFRelease(env, host);
    }
}

pub fn CFHostGetTypeID(env: &mut Environment) -> CFTypeID {
    let class: Class = env.objc.get_known_class("_touchHLE_CFHost", &mut env.mem);
    class.to_bits() as CFTypeID
}

// MARK: - Constructors

fn CFHostCreateWithName(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    hostname: CFTypeRef, // CFStringRef
) -> CFHostRef {
    if hostname.is_null() {
        return nil;
    }
    let name_str = ns_string::to_rust_string(env, hostname).into_owned();
    log_dbg!("CFHostCreateWithName({:?})", name_str);
    alloc_cfhost(env, Some(name_str), None)
}

fn CFHostCreateWithAddress(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    addr_data: CFTypeRef, // CFDataRef containing a sockaddr
) -> CFHostRef {
    if addr_data.is_null() {
        return alloc_cfhost(env, None, None);
    }

    let length = CFDataGetLength(env, addr_data);
    let ptr = CFDataGetBytePtr(env, addr_data);
    if length < std::mem::size_of::<sockaddr>() as i32 || ptr.is_null() {
        log!(
            "CFHostCreateWithAddress: CFData too small ({} bytes) or null, using empty host",
            length,
        );
        return alloc_cfhost(env, None, None);
    }

    let sa_ptr: crate::mem::ConstPtr<sockaddr> = ptr.cast();
    let sa: sockaddr = env.mem.read(sa_ptr);
    let address = if sockaddr_is_ipv4(&sa) {
        Some(sa.to_sockaddr_v4())
    } else {
        log!(
            "CFHostCreateWithAddress: non-IPv4 sockaddr (family {}), \
             no address stored",
            sockaddr_family(&sa),
        );
        None
    };
    log_dbg!("CFHostCreateWithAddress -> {:?}", address);
    alloc_cfhost(env, None, address)
}

fn CFHostCreateCopy(
    env: &mut Environment,
    _allocator: CFAllocatorRef,
    host: CFHostRef,
) -> CFHostRef {
    if host.is_null() {
        return nil;
    }
    let (name, address) = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        (h.name.clone(), h.address)
    };
    alloc_cfhost(env, name, address)
}

// MARK: - Resolution

fn perform_dns_lookup(name: &str) -> std::io::Result<Vec<SocketAddrV4>> {
    // Port 0 is a placeholder; only the addresses matter.
    let lookup_str = format!("{}:0", name);
    let addrs = lookup_str.to_socket_addrs()?;
    // IPv6 is not currently exposed to the guest (sockaddr is 16 bytes,
    // IPv4-only), so drop V6 results.
    Ok(addrs
        .filter_map(|addr| match addr {
            SocketAddr::V4(v4) => Some(v4),
            SocketAddr::V6(_) => None,
        })
        .collect())
}

fn fire_callout(env: &mut Environment, host: CFHostRef, info: CFHostInfoType) {
    let (callout, client_info) = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        (h.callout, h.client_info)
    };
    let Some(cb) = callout else {
        return;
    };
    // Callback signature:
    //   void (*CFHostClientCallBack)(CFHostRef, CFHostInfoType,
    //                                const CFStreamError*, void* info);
    // NULL CFStreamError* indicates success.
    let error_ptr: MutPtr<CFStreamError> = Ptr::null();
    log_dbg!(
        "CFHost: firing client callout {:?} host={:?} info={} client_info={:?}",
        cb,
        host,
        info,
        client_info,
    );
    let _: () = cb.call_from_host(env, (host, info, error_ptr, client_info));
}

fn CFHostStartInfoResolution(
    env: &mut Environment,
    host: CFHostRef,
    info: CFHostInfoType,
    error: MutPtr<CFStreamError>,
) -> bool {
    if host.is_null() {
        return false;
    }
    let info_idx = info as usize;
    if info_idx >= INFO_TYPE_COUNT {
        log!(
            "CFHostStartInfoResolution: invalid CFHostInfoType {}, returning false",
            info
        );
        write_stream_error(
            env,
            error,
            CFStreamError {
                domain: kCFStreamErrorDomainPOSIX,
                error: libc_EINVAL(),
            },
        );
        return false;
    }

    // Reject concurrent calls of the same kind.
    {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        if h.in_progress[info_idx] {
            log_dbg!(
                "CFHostStartInfoResolution: already resolving info {}, returning false",
                info
            );
            return false;
        }
    }
    {
        let h = env.objc.borrow_mut::<CFHostHostObject>(host);
        h.in_progress[info_idx] = true;
        h.cancelled[info_idx] = false;
    }

    let result = match info {
        kCFHostAddresses => start_addresses(env, host, error),
        kCFHostNames => start_names(env, host, error),
        kCFHostReachability => {
            // We don't model reachability probing; report success with no
            // data so callers that only branch on the Boolean are happy.
            env.objc.borrow_mut::<CFHostHostObject>(host).resolved[info_idx] = true;
            true
        }
        other => {
            // `info_idx < INFO_TYPE_COUNT` already filters out invalid values,
            // but we treat any future-added info type defensively rather than
            // crashing the host.
            log!(
                "Warning: CFHostStartInfoResolution: unsupported info type {}; returning false.",
                other
            );
            env.objc.borrow_mut::<CFHostHostObject>(host).in_progress[info_idx] = false;
            false
        }
    };

    env.objc.borrow_mut::<CFHostHostObject>(host).in_progress[info_idx] = false;

    // Notify the client if one was set and the resolution wasn't cancelled.
    // Apple fires the callout on both success and failure; on failure the
    // callout receives a non-NULL CFStreamError*, but we don't preserve the
    // per-resolution error across the async boundary in this minimal port,
    // so we just pass NULL and rely on the caller checking CFHostGetAddressing.
    let cancelled = env.objc.borrow::<CFHostHostObject>(host).cancelled[info_idx];
    if !cancelled {
        fire_callout(env, host, info);
    }
    result
}

fn start_addresses(env: &mut Environment, host: CFHostRef, error: MutPtr<CFStreamError>) -> bool {
    let (name_opt, static_addr) = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        (h.name.clone(), h.address)
    };

    if let Some(addr) = static_addr {
        // Host was created with a static address; nothing to resolve.
        let mut_h = env.objc.borrow_mut::<CFHostHostObject>(host);
        mut_h.resolved_addresses = vec![addr];
        mut_h.resolved[kCFHostAddresses as usize] = true;
        return true;
    }

    let Some(name_str) = name_opt else {
        log!("CFHostStartInfoResolution: host has no name or address");
        write_stream_error(
            env,
            error,
            CFStreamError {
                domain: kCFStreamErrorDomainPOSIX,
                error: libc_EINVAL(),
            },
        );
        return false;
    };

    log_dbg!(
        "CFHostStartInfoResolution(addresses): resolving {:?}",
        name_str
    );
    match perform_dns_lookup(&name_str) {
        Ok(resolved) => {
            log!(
                "CFHost: {:?} resolved to {} address(es)",
                name_str,
                resolved.len(),
            );
            let h = env.objc.borrow_mut::<CFHostHostObject>(host);
            h.resolved_addresses = resolved;
            h.resolved[kCFHostAddresses as usize] = true;
            true
        }
        Err(e) => {
            log!("CFHost: DNS lookup for {:?} failed: {}", name_str, e);
            write_stream_error(
                env,
                error,
                CFStreamError {
                    domain: kCFStreamErrorDomainPOSIX,
                    error: e.raw_os_error().unwrap_or(0),
                },
            );
            false
        }
    }
}

fn start_names(env: &mut Environment, host: CFHostRef, _error: MutPtr<CFStreamError>) -> bool {
    // Reverse DNS is rarely useful for touchHLE; report the name we were
    // created with, if any, so callers that inspect names don't see nil.
    let (name_opt, addr_opt) = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        (h.name.clone(), h.address)
    };
    let names: Vec<String> = match (name_opt, addr_opt) {
        (Some(name), _) => vec![name],
        (None, Some(addr)) => vec![format!("{}", addr.ip())],
        (None, None) => Vec::new(),
    };
    let h = env.objc.borrow_mut::<CFHostHostObject>(host);
    h.resolved_names = names;
    h.resolved[kCFHostNames as usize] = true;
    true
}

fn CFHostCancelInfoResolution(env: &mut Environment, host: CFHostRef, info: CFHostInfoType) {
    if host.is_null() {
        return;
    }
    let idx = info as usize;
    if idx >= INFO_TYPE_COUNT {
        return;
    }
    let h = env.objc.borrow_mut::<CFHostHostObject>(host);
    if h.in_progress[idx] {
        h.cancelled[idx] = true;
        log_dbg!(
            "CFHostCancelInfoResolution: marked info {} as cancelled",
            info
        );
    }
}

// MARK: - Run-loop scheduling

fn is_already_scheduled(entries: &[ScheduleEntry], run_loop: CFTypeRef, mode: &str) -> bool {
    entries
        .iter()
        .any(|e| e.run_loop == run_loop && e.mode == mode)
}

fn CFHostScheduleWithRunLoop(
    env: &mut Environment,
    host: CFHostRef,
    run_loop: CFTypeRef,
    run_loop_mode: CFTypeRef, // CFStringRef
) {
    if host.is_null() || run_loop.is_null() || run_loop_mode.is_null() {
        return;
    }
    let mode_str = ns_string::to_rust_string(env, run_loop_mode).into_owned();

    let already = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        is_already_scheduled(&h.scheduled, run_loop, &mode_str)
    };
    if already {
        log_dbg!(
            "CFHostScheduleWithRunLoop: already scheduled on rl={:?} mode={:?}",
            run_loop,
            mode_str
        );
        return;
    }

    let retained = CFRetain(env, run_loop);
    env.objc
        .borrow_mut::<CFHostHostObject>(host)
        .scheduled
        .push(ScheduleEntry {
            run_loop: retained,
            mode: mode_str.clone(),
        });
    log_dbg!(
        "CFHostScheduleWithRunLoop: host={:?} rl={:?} mode={:?}",
        host,
        run_loop,
        mode_str
    );
}

fn CFHostUnscheduleFromRunLoop(
    env: &mut Environment,
    host: CFHostRef,
    run_loop: CFTypeRef,
    run_loop_mode: CFTypeRef,
) {
    if host.is_null() || run_loop.is_null() || run_loop_mode.is_null() {
        return;
    }
    let mode_str = ns_string::to_rust_string(env, run_loop_mode).into_owned();

    let removed = {
        let h = env.objc.borrow_mut::<CFHostHostObject>(host);
        if let Some(pos) = h
            .scheduled
            .iter()
            .position(|e| e.run_loop == run_loop && e.mode == mode_str)
        {
            Some(h.scheduled.remove(pos))
        } else {
            None
        }
    };
    if let Some(entry) = removed {
        if !entry.run_loop.is_null() {
            CFRelease(env, entry.run_loop);
        }
        log_dbg!(
            "CFHostUnscheduleFromRunLoop: host={:?} rl={:?} mode={:?}",
            host,
            run_loop,
            mode_str
        );
    }
}

// MARK: - Client

fn CFHostSetClient(
    env: &mut Environment,
    host: CFHostRef,
    client_cb: MutVoidPtr, // CFHostClientCallBack (guest fn ptr)
    client_ctx: MutPtr<CFHostClientContext>,
) -> bool {
    if host.is_null() {
        return false;
    }

    // If a previous client was set, release its info before overwriting.
    let (prev_release, prev_info) = {
        let h = env.objc.borrow_mut::<CFHostHostObject>(host);
        let r = h.client_release.take();
        let i = std::mem::take(&mut h.client_info);
        h.callout = None;
        h.client_retain = None;
        (r, i)
    };
    if let Some(cb) = prev_release {
        if !prev_info.is_null() {
            let _: () = cb.call_from_host(env, (prev_info,));
        }
    }

    if client_cb.is_null() {
        // Passing NULL clears the client.
        log_dbg!("CFHostSetClient: cleared client for host {:?}", host);
        return true;
    }

    let callback = GuestFunction::from_addr_with_thumb_bit(client_cb.to_bits());

    let (info, retain_cb, release_cb) = if client_ctx.is_null() {
        (MutVoidPtr::null(), None, None)
    } else {
        let ctx: CFHostClientContext = env.mem.read(client_ctx);
        let info = ctx.info;
        let retain_cb = if ctx.retain.addr_with_thumb_bit() == 0 {
            None
        } else {
            Some(ctx.retain)
        };
        let release_cb = if ctx.release.addr_with_thumb_bit() == 0 {
            None
        } else {
            Some(ctx.release)
        };
        // Honour the retain callback so we keep the info pointer alive.
        let retained_info = if let (Some(cb), false) = (retain_cb, info.is_null()) {
            let result: MutVoidPtr = cb.call_from_host(env, (info,));
            result
        } else {
            info
        };
        (retained_info, retain_cb, release_cb)
    };

    let h = env.objc.borrow_mut::<CFHostHostObject>(host);
    h.callout = Some(callback);
    h.client_info = info;
    h.client_retain = retain_cb;
    h.client_release = release_cb;

    log_dbg!(
        "CFHostSetClient: host={:?} cb={:?} info={:?}",
        host,
        callback,
        info,
    );
    true
}

// MARK: - Info accessors

fn CFHostIsInfoResolved(env: &mut Environment, host: CFHostRef, info: CFHostInfoType) -> bool {
    if host.is_null() {
        return false;
    }
    let idx = info as usize;
    if idx >= INFO_TYPE_COUNT {
        return false;
    }
    env.objc.borrow::<CFHostHostObject>(host).resolved[idx]
}

fn CFHostGetAddressing(
    env: &mut Environment,
    host: CFHostRef,
    has_been_resolved: MutPtr<u8>, // Boolean*
) -> CFTypeRef {
    if host.is_null() {
        if !has_been_resolved.is_null() {
            env.mem.write(has_been_resolved, 0);
        }
        return nil;
    }

    let (resolved_flag, addresses) = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        (
            h.resolved[kCFHostAddresses as usize],
            h.resolved_addresses.clone(),
        )
    };

    if !has_been_resolved.is_null() {
        env.mem.write(has_been_resolved, resolved_flag as u8);
    }

    if !resolved_flag || addresses.is_empty() {
        return nil;
    }

    // Build a CFArray of CFData, each containing a `struct sockaddr`.
    let array = CFArrayCreateMutable(env, kCFAllocatorDefault, 0, ConstVoidPtr::null());
    for addr in &addresses {
        let sa = sockaddr::from_ipv4_parts(addr.ip().octets(), addr.port());
        let sa_size = std::mem::size_of::<sockaddr>() as u32;
        let buf: MutPtr<u8> = env.mem.alloc(sa_size).cast();
        env.mem.write(buf.cast::<sockaddr>(), sa);
        let data = CFDataCreate(
            env,
            kCFAllocatorDefault,
            buf.cast_const().cast(),
            sa_size as i32,
        );
        CFArrayAppendValue(env, array, data.cast().cast_const());
        CFRelease(env, data);
        env.mem.free(buf.cast());
    }
    array
}

fn CFHostGetNames(
    env: &mut Environment,
    host: CFHostRef,
    has_been_resolved: MutPtr<u8>, // Boolean*
) -> CFTypeRef {
    if host.is_null() {
        if !has_been_resolved.is_null() {
            env.mem.write(has_been_resolved, 0);
        }
        return nil;
    }

    let (resolved_flag, names) = {
        let h = env.objc.borrow::<CFHostHostObject>(host);
        (h.resolved[kCFHostNames as usize], h.resolved_names.clone())
    };

    if !has_been_resolved.is_null() {
        env.mem.write(has_been_resolved, resolved_flag as u8);
    }

    if !resolved_flag || names.is_empty() {
        return nil;
    }

    let array = CFArrayCreateMutable(env, kCFAllocatorDefault, 0, ConstVoidPtr::null());
    for name in &names {
        let s = ns_string::from_rust_string(env, name.clone());
        CFArrayAppendValue(env, array, s.cast().cast_const());
        release(env, s);
    }
    array
}

fn CFHostGetReachability(
    env: &mut Environment,
    host: CFHostRef,
    has_been_resolved: MutPtr<u8>, // Boolean*
) -> CFTypeRef {
    if host.is_null() {
        if !has_been_resolved.is_null() {
            env.mem.write(has_been_resolved, 0);
        }
        return nil;
    }
    let resolved_flag =
        env.objc.borrow::<CFHostHostObject>(host).resolved[kCFHostReachability as usize];
    if !has_been_resolved.is_null() {
        env.mem.write(has_been_resolved, resolved_flag as u8);
    }
    // We don't model actual reachability flags.
    nil
}

// MARK: - Small helpers

fn libc_EINVAL() -> i32 {
    22
}

/// Reads `sockaddr`'s `sa_len` byte without constructing a misaligned
/// reference (the struct is `repr(C, packed)`).
fn sockaddr_len(sa: &sockaddr) -> u8 {
    let ptr = sa as *const sockaddr as *const u8;
    // SAFETY: sockaddr is 16 bytes so byte 0 is in-bounds.
    unsafe { *ptr }
}

/// Reads `sockaddr`'s `sa_family` byte without constructing a misaligned
/// reference.
fn sockaddr_family(sa: &sockaddr) -> u8 {
    let ptr = sa as *const sockaddr as *const u8;
    // SAFETY: sockaddr is 16 bytes so byte 1 is in-bounds.
    unsafe { *ptr.add(1) }
}

fn sockaddr_is_ipv4(sa: &sockaddr) -> bool {
    // AF_INET on Darwin is 2.
    sockaddr_family(sa) == 2 && (sockaddr_len(sa) == 16 || sockaddr_len(sa) == 0)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFHostRetain(_)),
    export_c_func!(CFHostRelease(_)),
    export_c_func!(CFHostCreateWithName(_, _)),
    export_c_func!(CFHostCreateWithAddress(_, _)),
    export_c_func!(CFHostCreateCopy(_, _)),
    export_c_func!(CFHostStartInfoResolution(_, _, _)),
    export_c_func!(CFHostCancelInfoResolution(_, _)),
    export_c_func!(CFHostScheduleWithRunLoop(_, _, _)),
    export_c_func!(CFHostUnscheduleFromRunLoop(_, _, _)),
    export_c_func!(CFHostSetClient(_, _, _)),
    export_c_func!(CFHostGetAddressing(_, _)),
    export_c_func!(CFHostGetNames(_, _)),
    export_c_func!(CFHostGetReachability(_, _)),
    export_c_func!(CFHostIsInfoResolved(_, _)),
    export_c_func!(CFHostGetTypeID()),
];
