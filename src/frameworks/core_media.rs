/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `CoreMedia.framework/CoreMedia`.
//!
//! On iOS, CoreMedia provides time-related types (`CMTime`, `CMTimeRange`),
//! sample buffer plumbing (`CMSampleBufferRef`), and format descriptions used
//! mostly by AVFoundation. Apps that link against CoreMedia (directly or
//! transitively, e.g. via AVFoundation cutscene playback) put the path
//! `/System/Library/Frameworks/CoreMedia.framework/CoreMedia` in their Mach-O
//! load commands.
//!
//! Without a [crate::dyld::HostDylib] entry for that path, touchHLE prints a
//! `Warning: app binary depends on unimplemented or missing dylib
//! "/System/Library/Frameworks/CoreMedia.framework/CoreMedia"` at startup,
//! which can spook users into reporting otherwise-fine apps as broken (e.g.
//! HyperHLE appdb report #22, GhostToasters).
//!
//! This stub exists so that the dependency is recognized and the warning is
//! suppressed. The few CoreMedia functions touchHLE currently implements
//! (`CMSampleBufferGetImageBuffer`, `CMSampleBufferDataIsReady`, …) are
//! registered with [crate::frameworks::core_video] for historical reasons;
//! `dyld` searches all framework `function_exports` regardless of which
//! dylib they were declared under, so the binding still resolves correctly
//! whether the app links CoreMedia or CoreVideo.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstVoidPtr, MutPtr};
use crate::Environment;

/// `CMTime` is a 24-byte struct (`int64_t value`, `int32_t timescale`,
/// `uint32_t flags`, `int64_t epoch`). `kCMTimeZero` is the all-zero
/// constant — `{value:0, timescale:0, flags:0, epoch:0}`. Apps reach the
/// symbol by Mach-O lookup and either compare against it for equality
/// or copy it as a starting point. Allocate the layout once per access
/// site; CFTime's other "well-known" constants (kCMTimeInvalid,
/// kCMTimePositiveInfinity, …) fit the same template, so we expose
/// each as a separate slot.
fn cm_time_zero(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    p.cast().cast_const()
}

/// `kCMTimeInvalid` has flags=0 (kCMTimeFlags_Valid bit clear). Same
/// 24-byte layout as kCMTimeZero, every byte zero.
fn cm_time_invalid(env: &mut Environment) -> ConstVoidPtr {
    cm_time_zero(env)
}

/// `kCMTimeIndefinite`: value=0, timescale=0, flags=kCMTimeFlags_Valid|
/// kCMTimeFlags_Indefinite (=0x3), epoch=0. Apps usually only check for
/// (flags & kCMTimeFlags_Valid) and ignore the rest, so a flags-only
/// difference from kCMTimeZero is enough.
fn cm_time_indefinite(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    // Bytes 12..16 are the `flags` field (after value:i64, timescale:i32).
    env.mem.write(p + 12u32, 0x03);
    p.cast().cast_const()
}

/// `kCMTimePositiveInfinity` and `kCMTimeNegativeInfinity` use flags
/// `kCMTimeFlags_Valid|kCMTimeFlags_PositiveInfinity` (0x5) and
/// `kCMTimeFlags_Valid|kCMTimeFlags_NegativeInfinity` (0x9) respectively.
fn cm_time_positive_infinity(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    env.mem.write(p + 12u32, 0x05);
    p.cast().cast_const()
}

fn cm_time_negative_infinity(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(24).cast();
    for i in 0..24 {
        env.mem.write(p + i, 0);
    }
    env.mem.write(p + 12u32, 0x09);
    p.cast().cast_const()
}

/// `kCMTimingInfoInvalid` is a 56-byte `CMSampleTimingInfo` struct
/// (3 × `CMTime` = 72 bytes — actually 72, since each CMTime is 24).
/// Apple `CMSampleBuffer.h` defines all three CMTime fields as
/// `kCMTimeInvalid`, i.e. every byte zero. Apps memcpy this whole
/// struct as a starting point or compare individual fields to
/// `kCMTimeInvalid`. The published constant is an `extern const`.
/// <https://developer.apple.com/documentation/coremedia/kcmtiminginfo_invalid>
fn cm_timing_info_invalid(env: &mut Environment) -> ConstVoidPtr {
    let p: MutPtr<u8> = env.mem.alloc(72).cast();
    for i in 0..72 {
        env.mem.write(p + i, 0);
    }
    p.cast().cast_const()
}

pub const CONSTANTS: ConstantExports = &[
    ("_kCMTimeZero", HostConstant::Custom(cm_time_zero)),
    ("_kCMTimeInvalid", HostConstant::Custom(cm_time_invalid)),
    (
        "_kCMTimeIndefinite",
        HostConstant::Custom(cm_time_indefinite),
    ),
    (
        "_kCMTimePositiveInfinity",
        HostConstant::Custom(cm_time_positive_infinity),
    ),
    (
        "_kCMTimeNegativeInfinity",
        HostConstant::Custom(cm_time_negative_infinity),
    ),
    (
        "_kCMTimingInfoInvalid",
        HostConstant::Custom(cm_timing_info_invalid),
    ),
    // -----------------------------------------------------------------
    // `kCMSampleAttachmentKey_*` — CFStringRef keys used with
    // `CMSampleBufferGetSampleAttachmentsArray()`. Apple ships them as
    // `extern const CFStringRef`; their literal values are the
    // canonical CMSampleBuffer.h tags. Apps reach them through string
    // identity / `CFEqual()` comparisons.
    // <https://developer.apple.com/documentation/coremedia/kcmsampleattachmentkey_notsync>
    // -----------------------------------------------------------------
    (
        "_kCMSampleAttachmentKey_NotSync",
        HostConstant::NSString("NotSync"),
    ),
    (
        "_kCMSampleAttachmentKey_PartialSync",
        HostConstant::NSString("PartialSync"),
    ),
    (
        "_kCMSampleAttachmentKey_HasRedundantCoding",
        HostConstant::NSString("HasRedundantCoding"),
    ),
    (
        "_kCMSampleAttachmentKey_IsDependedOnByOthers",
        HostConstant::NSString("IsDependedOnByOthers"),
    ),
    (
        "_kCMSampleAttachmentKey_DependsOnOthers",
        HostConstant::NSString("DependsOnOthers"),
    ),
    (
        "_kCMSampleAttachmentKey_EarlierDisplayTimesAllowed",
        HostConstant::NSString("EarlierDisplayTimesAllowed"),
    ),
    (
        "_kCMSampleAttachmentKey_DisplayImmediately",
        HostConstant::NSString("DisplayImmediately"),
    ),
    (
        "_kCMSampleAttachmentKey_DoNotDisplay",
        HostConstant::NSString("DoNotDisplay"),
    ),
];

/// `CMTimeMake(int64_t value, int32_t timescale)` returns a `CMTime` with
/// `kCMTimeFlags_Valid` set, `epoch == 0` and the supplied value /
/// timescale. On 32-bit ARM iOS the 24-byte struct is returned via a
/// hidden first sret pointer (the AAPCS "indirect result location"
/// register r0); the caller supplies a pointer to writable storage,
/// and the C-visible arguments shift one slot to the right. Apple
/// inlined this helper in `<CoreMedia/CMTime.h>` before iOS 7.1 and
/// began exporting an out-of-line `_CMTimeMake` symbol shortly after,
/// which is the symbol the touchHLE-targeted apps reference here.
/// <https://developer.apple.com/documentation/coremedia/cmtimemake(_:_:)>
fn CMTimeMake(env: &mut Environment, out: MutPtr<u8>, value: i64, timescale: i32) -> MutPtr<u8> {
    // Apple ARM 32-bit ABI lays out `CMTime` as:
    //   offset  0: int64_t  value      (8 bytes, naturally aligned)
    //   offset  8: int32_t  timescale  (4 bytes)
    //   offset 12: uint32_t flags      (4 bytes; bit 0 = kCMTimeFlags_Valid)
    //   offset 16: int64_t  epoch      (8 bytes)  → total = 24 bytes
    // <https://developer.apple.com/documentation/coremedia/cmtime>
    let value_bytes = value.to_le_bytes();
    for (i, b) in value_bytes.iter().enumerate() {
        env.mem.write(out + (i as u32), *b);
    }
    let timescale_bytes = timescale.to_le_bytes();
    for (i, b) in timescale_bytes.iter().enumerate() {
        env.mem.write(out + 8u32 + (i as u32), *b);
    }
    // `flags = kCMTimeFlags_Valid` (0x1).
    env.mem.write(out + 12u32, 0x01);
    env.mem.write(out + 13u32, 0x00);
    env.mem.write(out + 14u32, 0x00);
    env.mem.write(out + 15u32, 0x00);
    // `epoch = 0` — zero out bytes 16..24.
    for i in 16..24u32 {
        env.mem.write(out + i, 0);
    }
    out
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CMTimeMake(_, _, _))];
