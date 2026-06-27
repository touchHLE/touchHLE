/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSProcessInfo`.
//!
//! Apple references used:
//! - <https://developer.apple.com/documentation/foundation/nsprocessinfo>
//! - <https://developer.apple.com/documentation/foundation/nsoperatingsystemversion>
//! - `Foundation/NSProcessInfo.h` declares `NSOperatingSystemVersion` as a
//!   struct of three `NSInteger` fields (`majorVersion`, `minorVersion`,
//!   `patchVersion`).
//!
//! On 32-bit iOS `NSInteger` is `int32_t` (4 bytes), so the struct is 12
//! bytes total and is returned by value via the AAPCS struct-return ABI
//! (i.e. via the stret variant of `objc_msgSend`).

use super::NSTimeInterval;
use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::frameworks::foundation::ns_string;
use crate::libc::mach::host::PHYSICAL_MEMORY;
use crate::mem::SafeRead;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports};
use crate::Environment;
use std::time::Instant;

/// `NSOperatingSystemVersion` from `Foundation/NSProcessInfo.h`.
///
/// ```c
/// typedef struct {
///     NSInteger majorVersion;
///     NSInteger minorVersion;
///     NSInteger patchVersion;
/// } NSOperatingSystemVersion;
/// ```
///
/// On 32-bit iOS, `NSInteger` is a 32-bit signed integer.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(C, packed)]
pub struct NSOperatingSystemVersion {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}
unsafe impl SafeRead for NSOperatingSystemVersion {}
impl_GuestRet_for_large_struct!(NSOperatingSystemVersion);
impl GuestArg for NSOperatingSystemVersion {
    const REG_COUNT: usize = 3;
    fn from_regs(regs: &[u32]) -> Self {
        NSOperatingSystemVersion {
            major: GuestArg::from_regs(&regs[0..1]),
            minor: GuestArg::from_regs(&regs[1..2]),
            patch: GuestArg::from_regs(&regs[2..3]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        self.major.to_regs(&mut regs[0..1]);
        self.minor.to_regs(&mut regs[1..2]);
        self.patch.to_regs(&mut regs[2..3]);
    }
}

#[derive(Default)]
pub struct State {
    /// `NSProcessInfo*`
    process_info: Option<id>,
}

fn assert_process_info_singleton(env: &mut Environment, this: id) {
    assert_eq!(
        this,
        env.framework_state
            .foundation
            .ns_process_info
            .process_info
            .unwrap()
    );
}

/// Fake OS version used when the app queries the host system version.
///
/// touchHLE targets early iPhone OS apps; we report iOS 12.0.0 so that any
/// iOS 8+ feature gate (which is the floor for many third-party SDKs that
/// query `operatingSystemVersion`) passes without triggering the
/// "unsupported version" path inside the app.
const OS_VERSION_MAJOR: i32 = 12;
const OS_VERSION_MINOR: i32 = 0;
const OS_VERSION_PATCH: i32 = 0;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

// =========================================================================
// MARK: - Singleton
// =========================================================================

+ (id)processInfo {
    if let Some(existing) = env.framework_state.foundation.ns_process_info.process_info {
        existing
    } else {
        let process_info: id = msg![env; this new];
        env.framework_state.foundation.ns_process_info.process_info = Some(process_info);
        process_info
    }
}

// =========================================================================
// MARK: - Process identity
// =========================================================================

- (id)processName {
    assert_process_info_singleton(env, this);
    let main_bundle: id = msg_class![env; NSBundle mainBundle];
    let name_key: id = ns_string::get_static_str(env, "CFBundleName");
    msg![env; main_bundle objectForInfoDictionaryKey:name_key]
}

- (())setProcessName:(id)_name {
    assert_process_info_singleton(env, this);
    log!("TODO: [NSProcessInfo setProcessName:] — ignored");
}

- (i32)processIdentifier {
    assert_process_info_singleton(env, this);
    1234
}

- (id)globallyUniqueString {
    assert_process_info_singleton(env, this);
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let uptime_ns = Instant::now()
        .duration_since(env.startup_time)
        .as_nanos() as u64;
    let s = format!("1234-{}-{}", uptime_ns, n);
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

// =========================================================================
// MARK: - Host / environment
// =========================================================================

- (id)hostName {
    assert_process_info_singleton(env, this);
    ns_string::get_static_str(env, "touchHLE-host.local")
}

- (id)arguments {
    assert_process_info_singleton(env, this);
    msg_class![env; NSArray array]
}

- (id)environment {
    assert_process_info_singleton(env, this);
    msg_class![env; NSDictionary dictionary]
}

// =========================================================================
// MARK: - Hardware / memory
// =========================================================================

- (u64)physicalMemory {
    assert_process_info_singleton(env, this);
    PHYSICAL_MEMORY.into()
}

- (u32)processorCount {
    assert_process_info_singleton(env, this);
    1
}

- (u32)activeProcessorCount {
    assert_process_info_singleton(env, this);
    1
}

// =========================================================================
// MARK: - Uptime
// =========================================================================

- (NSTimeInterval)systemUptime {
    assert_process_info_singleton(env, this);
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

// =========================================================================
// MARK: - OS version
// =========================================================================

// `- (NSOperatingSystemVersion)operatingSystemVersion` (iOS 8+).
//
// Documented at
// <https://developer.apple.com/documentation/foundation/nsprocessinfo/1410031-operatingsystemversion>.
//
// The selector returns `NSOperatingSystemVersion` *by value*. Because the
// struct is larger than the AAPCS register-return limit (4 bytes on
// 32-bit ARM), the compiler emits `objc_msgSend_stret`, passing a hidden
// pointer in `r0` for the struct destination. Returning `id` here (as the
// old stub did) caused the receiver to be mis-mapped to that destination
// pointer, which produced the assertion failure observed in Bloons TD 5.
- (NSOperatingSystemVersion)operatingSystemVersion {
    assert_process_info_singleton(env, this);
    NSOperatingSystemVersion {
        major: OS_VERSION_MAJOR,
        minor: OS_VERSION_MINOR,
        patch: OS_VERSION_PATCH,
    }
}

- (id)operatingSystemVersionString {
    assert_process_info_singleton(env, this);
    let s = format!(
        "Version {}.{}.{} (Build 16A366)",
        OS_VERSION_MAJOR, OS_VERSION_MINOR, OS_VERSION_PATCH
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

// `- (BOOL)isOperatingSystemAtLeastVersion:(NSOperatingSystemVersion)version`
// (iOS 8+).
//
// Documented at
// <https://developer.apple.com/documentation/foundation/nsprocessinfo/1417380-isoperatingsystematleastversion>.
//
// Takes a single `NSOperatingSystemVersion` struct by value. On 32-bit
// ARM AAPCS the 12-byte struct is split across `r2`, `r3` and one stack
// slot (after the implicit `self`/`_cmd`). The previous declaration used
// three separate `u64` parameters with the wrong selector
// (`...:minor:patch:`), so it was unreachable from real apps.
- (bool)isOperatingSystemAtLeastVersion:(NSOperatingSystemVersion)version {
    assert_process_info_singleton(env, this);
    let NSOperatingSystemVersion { major, minor, patch } = version;
    if major != OS_VERSION_MAJOR { return major < OS_VERSION_MAJOR; }
    if minor != OS_VERSION_MINOR { return minor < OS_VERSION_MINOR; }
    patch <= OS_VERSION_PATCH
}

// =========================================================================
// MARK: - Thermal state (iOS 11+)
// =========================================================================

// NSProcessInfoThermalStateNominal = 0
- (i64)thermalState {
    assert_process_info_singleton(env, this);
    0
}

// =========================================================================
// MARK: - Low Power Mode (iOS 9+)
// =========================================================================

- (bool)isLowPowerModeEnabled {
    assert_process_info_singleton(env, this);
    false
}

// =========================================================================
// MARK: - Activity assertions (iOS 7+)
// =========================================================================

- (id)beginActivityWithOptions:(u64)_options reason:(id)_reason {
    assert_process_info_singleton(env, this);
    log!("TODO: [NSProcessInfo beginActivityWithOptions:reason:] — returning stub token");
    this
}

- (())endActivity:(id)_activity {
    assert_process_info_singleton(env, this);
}

- (())performActivityWithOptions:(u64)_options
                          reason:(id)_reason
                      usingBlock:(id)block {
    assert_process_info_singleton(env, this);
    let _: () = msg![env; block invoke];
}

// =========================================================================
// MARK: - Sudden termination (macOS; silently ignored on iOS)
// =========================================================================

- (())disableSuddenTermination {
    assert_process_info_singleton(env, this);
}

- (())enableSuddenTermination {
    assert_process_info_singleton(env, this);
}

- (())disableAutomaticTermination:(id)_reason {
    assert_process_info_singleton(env, this);
}

- (())enableAutomaticTermination:(id)_reason {
    assert_process_info_singleton(env, this);
}

- (bool)automaticTerminationSupportEnabled {
    assert_process_info_singleton(env, this);
    false
}

- (())setAutomaticTerminationSupportEnabled:(bool)_enabled {
    assert_process_info_singleton(env, this);
}

@end

};
