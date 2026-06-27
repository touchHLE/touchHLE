/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AVFoundation Capture API.
//!
//! Implements `AVCaptureSession`, `AVCaptureDevice`, `AVCaptureDeviceInput`,
//! `AVCaptureVideoDataOutput`, `AVCaptureMetadataOutput`, and
//! `AVCaptureVideoPreviewLayer`. The implementation is **not** a real camera
//! pipeline yet; it is the high-level Objective-C surface plus a pluggable
//! backend (see [crate::camera]). When no backend is available, a synthetic
//! frame source is used so that apps relying on the camera API to even reach
//! their main UI can boot.
//!
//! This unblocks AR / barcode-reader apps such as LEGO Ninjago Spinjitzu
//! Scavenger Hunt that put their primary view controller on top of an
//! `AVCaptureVideoPreviewLayer` and never render anything else if the
//! preview layer is missing — they appear as a black screen even though
//! their render loop is alive.
//!
//! See [crate::camera] for the actual frame source.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::{ns_array, ns_string};
use crate::objc::{
    autorelease, id, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr, TrivialHostObject,
};

// ============================================================================
// MARK: - Constants
// ============================================================================
//
// All AVCapture* string constants are exported as NSString. Apps reach them
// through Mach-O symbol lookup (e.g. `extern NSString * const AVMediaTypeVideo`)
// which `dyld::HostConstant::NSString` handles.

// AVMediaType.h — per Apple's
// <https://developer.apple.com/documentation/avfoundation/avmediatype>.
// The literal value of `AVMediaTypeVideo` etc. is the corresponding QuickTime
// FourCC ("vide", "soun", ...). Apps compare these by pointer or via
// `[NSString isEqualToString:]`, so the only requirement is that each
// constant resolves to a distinct, non-NULL NSString.
pub const AVMediaTypeVideo: &str = "vide";
pub const AVMediaTypeAudio: &str = "soun";
pub const AVMediaTypeText: &str = "text";
pub const AVMediaTypeMuxed: &str = "muxx";
pub const AVMediaTypeClosedCaption: &str = "clcp";
pub const AVMediaTypeSubtitle: &str = "sbtl";
pub const AVMediaTypeTimecode: &str = "tmcd";
pub const AVMediaTypeMetadata: &str = "meta";
pub const AVMediaTypeMetadataObject: &str = "meta-object";
pub const AVMediaTypeDepthData: &str = "dpth";

// AVMediaFormat.h — `AVMediaCharacteristic*` per Apple's
// <https://developer.apple.com/documentation/avfoundation/avmediacharacteristic>.
// These tag a `AVMediaSelectionOption` / `AVAssetTrack` with a high-level
// purpose (audible, visual, legible, ...). The string value is the literal
// reverse-DNS identifier Apple ships ("public.audio", "public.visual", ...).
pub const AVMediaCharacteristicVisual: &str = "public.visual";
pub const AVMediaCharacteristicAudible: &str = "public.audible";
pub const AVMediaCharacteristicLegible: &str = "public.legible";
pub const AVMediaCharacteristicFrameBased: &str = "public.frame-based";
pub const AVMediaCharacteristicIsMainProgramContent: &str =
    "public.main-program-content";
pub const AVMediaCharacteristicIsAuxiliaryContent: &str =
    "public.auxiliary-content";
pub const AVMediaCharacteristicContainsOnlyForcedSubtitles: &str =
    "public.subtitles.forced-only";
pub const AVMediaCharacteristicTranscribesSpokenDialogForAccessibility: &str =
    "public.accessibility.transcribes-spoken-dialog";
pub const AVMediaCharacteristicDescribesMusicAndSoundForAccessibility: &str =
    "public.accessibility.describes-music-and-sound";
pub const AVMediaCharacteristicEasyToRead: &str = "public.easy-to-read";
pub const AVMediaCharacteristicDescribesVideoForAccessibility: &str =
    "public.accessibility.describes-video";
pub const AVMediaCharacteristicLanguageTranslation: &str =
    "public.translation";
pub const AVMediaCharacteristicDubbedTranslation: &str =
    "public.translation.dubbed";
pub const AVMediaCharacteristicVoiceOverTranslation: &str =
    "public.translation.voice-over";

// AVCaptureSession presets.
pub const AVCaptureSessionPresetPhoto: &str = "AVCaptureSessionPresetPhoto";
pub const AVCaptureSessionPresetHigh: &str = "AVCaptureSessionPresetHigh";
pub const AVCaptureSessionPresetMedium: &str = "AVCaptureSessionPresetMedium";
pub const AVCaptureSessionPresetLow: &str = "AVCaptureSessionPresetLow";
pub const AVCaptureSessionPreset352x288: &str = "AVCaptureSessionPreset352x288";
pub const AVCaptureSessionPreset640x480: &str = "AVCaptureSessionPreset640x480";
pub const AVCaptureSessionPreset1280x720: &str = "AVCaptureSessionPreset1280x720";
pub const AVCaptureSessionPreset1920x1080: &str = "AVCaptureSessionPreset1920x1080";

// AVCaptureSession notifications & error keys.
pub const AVCaptureSessionRuntimeErrorNotification: &str =
    "AVCaptureSessionRuntimeErrorNotification";
pub const AVCaptureSessionDidStartRunningNotification: &str =
    "AVCaptureSessionDidStartRunningNotification";
pub const AVCaptureSessionDidStopRunningNotification: &str =
    "AVCaptureSessionDidStopRunningNotification";
pub const AVCaptureSessionWasInterruptedNotification: &str =
    "AVCaptureSessionWasInterruptedNotification";
pub const AVCaptureSessionInterruptionEndedNotification: &str =
    "AVCaptureSessionInterruptionEndedNotification";
pub const AVCaptureSessionErrorKey: &str = "AVCaptureSessionErrorKey";

// AVCaptureVideoPreviewLayer gravity constants.
//
// These are also defined on AVPlayerLayer/AVCaptureVideoPreviewLayer; we just
// re-export them here under their AVLayer* names since the iOS headers had
// both AVLayerVideoGravity* and AVCaptureVideoPreviewLayerVideoGravity* keys.
pub const AVLayerVideoGravityResize: &str = "AVLayerVideoGravityResize";
pub const AVLayerVideoGravityResizeAspect: &str = "AVLayerVideoGravityResizeAspect";
pub const AVLayerVideoGravityResizeAspectFill: &str = "AVLayerVideoGravityResizeAspectFill";

// Metadata object types (AVMetadataObject.h, partial).
pub const AVMetadataObjectTypeFace: &str = "face";
pub const AVMetadataObjectTypeQRCode: &str = "org.iso.QRCode";
pub const AVMetadataObjectTypeAztecCode: &str = "org.iso.Aztec";
pub const AVMetadataObjectTypeCode128Code: &str = "org.iso.Code128";
pub const AVMetadataObjectTypeCode39Code: &str = "org.iso.Code39";
pub const AVMetadataObjectTypeCode93Code: &str = "com.intermec.Code93";
pub const AVMetadataObjectTypeEAN13Code: &str = "org.gs1.EAN-13";
pub const AVMetadataObjectTypeEAN8Code: &str = "org.gs1.EAN-8";
pub const AVMetadataObjectTypePDF417Code: &str = "org.iso.PDF417";
pub const AVMetadataObjectTypeUPCECode: &str = "org.gs1.UPC-E";

pub const CONSTANTS: ConstantExports = &[
    (
        "_AVMediaTypeVideo",
        HostConstant::NSString(AVMediaTypeVideo),
    ),
    (
        "_AVMediaTypeAudio",
        HostConstant::NSString(AVMediaTypeAudio),
    ),
    ("_AVMediaTypeText", HostConstant::NSString(AVMediaTypeText)),
    (
        "_AVMediaTypeMuxed",
        HostConstant::NSString(AVMediaTypeMuxed),
    ),
    (
        "_AVCaptureSessionPresetPhoto",
        HostConstant::NSString(AVCaptureSessionPresetPhoto),
    ),
    (
        "_AVCaptureSessionPresetHigh",
        HostConstant::NSString(AVCaptureSessionPresetHigh),
    ),
    (
        "_AVCaptureSessionPresetMedium",
        HostConstant::NSString(AVCaptureSessionPresetMedium),
    ),
    (
        "_AVCaptureSessionPresetLow",
        HostConstant::NSString(AVCaptureSessionPresetLow),
    ),
    (
        "_AVCaptureSessionPreset352x288",
        HostConstant::NSString(AVCaptureSessionPreset352x288),
    ),
    (
        "_AVCaptureSessionPreset640x480",
        HostConstant::NSString(AVCaptureSessionPreset640x480),
    ),
    (
        "_AVCaptureSessionPreset1280x720",
        HostConstant::NSString(AVCaptureSessionPreset1280x720),
    ),
    (
        "_AVCaptureSessionPreset1920x1080",
        HostConstant::NSString(AVCaptureSessionPreset1920x1080),
    ),
    (
        "_AVCaptureSessionRuntimeErrorNotification",
        HostConstant::NSString(AVCaptureSessionRuntimeErrorNotification),
    ),
    (
        "_AVCaptureSessionDidStartRunningNotification",
        HostConstant::NSString(AVCaptureSessionDidStartRunningNotification),
    ),
    (
        "_AVCaptureSessionDidStopRunningNotification",
        HostConstant::NSString(AVCaptureSessionDidStopRunningNotification),
    ),
    (
        "_AVCaptureSessionWasInterruptedNotification",
        HostConstant::NSString(AVCaptureSessionWasInterruptedNotification),
    ),
    (
        "_AVCaptureSessionInterruptionEndedNotification",
        HostConstant::NSString(AVCaptureSessionInterruptionEndedNotification),
    ),
    (
        "_AVCaptureSessionErrorKey",
        HostConstant::NSString(AVCaptureSessionErrorKey),
    ),
    (
        "_AVLayerVideoGravityResize",
        HostConstant::NSString(AVLayerVideoGravityResize),
    ),
    (
        "_AVLayerVideoGravityResizeAspect",
        HostConstant::NSString(AVLayerVideoGravityResizeAspect),
    ),
    (
        "_AVLayerVideoGravityResizeAspectFill",
        HostConstant::NSString(AVLayerVideoGravityResizeAspectFill),
    ),
    (
        "_AVMetadataObjectTypeFace",
        HostConstant::NSString(AVMetadataObjectTypeFace),
    ),
    (
        "_AVMetadataObjectTypeQRCode",
        HostConstant::NSString(AVMetadataObjectTypeQRCode),
    ),
    (
        "_AVMetadataObjectTypeAztecCode",
        HostConstant::NSString(AVMetadataObjectTypeAztecCode),
    ),
    (
        "_AVMetadataObjectTypeCode128Code",
        HostConstant::NSString(AVMetadataObjectTypeCode128Code),
    ),
    (
        "_AVMetadataObjectTypeCode39Code",
        HostConstant::NSString(AVMetadataObjectTypeCode39Code),
    ),
    (
        "_AVMetadataObjectTypeCode93Code",
        HostConstant::NSString(AVMetadataObjectTypeCode93Code),
    ),
    (
        "_AVMetadataObjectTypeEAN13Code",
        HostConstant::NSString(AVMetadataObjectTypeEAN13Code),
    ),
    (
        "_AVMetadataObjectTypeEAN8Code",
        HostConstant::NSString(AVMetadataObjectTypeEAN8Code),
    ),
    (
        "_AVMetadataObjectTypePDF417Code",
        HostConstant::NSString(AVMetadataObjectTypePDF417Code),
    ),
    (
        "_AVMetadataObjectTypeUPCECode",
        HostConstant::NSString(AVMetadataObjectTypeUPCECode),
    ),
    // Additional AVMediaType members.
    (
        "_AVMediaTypeClosedCaption",
        HostConstant::NSString(AVMediaTypeClosedCaption),
    ),
    (
        "_AVMediaTypeSubtitle",
        HostConstant::NSString(AVMediaTypeSubtitle),
    ),
    (
        "_AVMediaTypeTimecode",
        HostConstant::NSString(AVMediaTypeTimecode),
    ),
    (
        "_AVMediaTypeMetadata",
        HostConstant::NSString(AVMediaTypeMetadata),
    ),
    (
        "_AVMediaTypeMetadataObject",
        HostConstant::NSString(AVMediaTypeMetadataObject),
    ),
    (
        "_AVMediaTypeDepthData",
        HostConstant::NSString(AVMediaTypeDepthData),
    ),
    // AVMediaCharacteristic.
    (
        "_AVMediaCharacteristicVisual",
        HostConstant::NSString(AVMediaCharacteristicVisual),
    ),
    (
        "_AVMediaCharacteristicAudible",
        HostConstant::NSString(AVMediaCharacteristicAudible),
    ),
    (
        "_AVMediaCharacteristicLegible",
        HostConstant::NSString(AVMediaCharacteristicLegible),
    ),
    (
        "_AVMediaCharacteristicFrameBased",
        HostConstant::NSString(AVMediaCharacteristicFrameBased),
    ),
    (
        "_AVMediaCharacteristicIsMainProgramContent",
        HostConstant::NSString(AVMediaCharacteristicIsMainProgramContent),
    ),
    (
        "_AVMediaCharacteristicIsAuxiliaryContent",
        HostConstant::NSString(AVMediaCharacteristicIsAuxiliaryContent),
    ),
    (
        "_AVMediaCharacteristicContainsOnlyForcedSubtitles",
        HostConstant::NSString(AVMediaCharacteristicContainsOnlyForcedSubtitles),
    ),
    (
        "_AVMediaCharacteristicTranscribesSpokenDialogForAccessibility",
        HostConstant::NSString(
            AVMediaCharacteristicTranscribesSpokenDialogForAccessibility,
        ),
    ),
    (
        "_AVMediaCharacteristicDescribesMusicAndSoundForAccessibility",
        HostConstant::NSString(
            AVMediaCharacteristicDescribesMusicAndSoundForAccessibility,
        ),
    ),
    (
        "_AVMediaCharacteristicEasyToRead",
        HostConstant::NSString(AVMediaCharacteristicEasyToRead),
    ),
    (
        "_AVMediaCharacteristicDescribesVideoForAccessibility",
        HostConstant::NSString(
            AVMediaCharacteristicDescribesVideoForAccessibility,
        ),
    ),
    (
        "_AVMediaCharacteristicLanguageTranslation",
        HostConstant::NSString(AVMediaCharacteristicLanguageTranslation),
    ),
    (
        "_AVMediaCharacteristicDubbedTranslation",
        HostConstant::NSString(AVMediaCharacteristicDubbedTranslation),
    ),
    (
        "_AVMediaCharacteristicVoiceOverTranslation",
        HostConstant::NSString(AVMediaCharacteristicVoiceOverTranslation),
    ),
];

// ============================================================================
// MARK: - Per-app framework state
// ============================================================================

#[derive(Default)]
pub struct State {
    /// Live `AVCaptureSession` instances. We track them so the camera
    /// backend can deliver frames to whichever sessions are running.
    pub running_sessions: Vec<id>,
}

// ============================================================================
// MARK: - Host objects
// ============================================================================

#[derive(Default)]
struct AVCaptureSessionHostObject {
    inputs: Vec<id>,
    outputs: Vec<id>,
    preset: id,
    running: bool,
    interrupted: bool,
}
impl HostObject for AVCaptureSessionHostObject {}

#[derive(Default)]
struct AVCaptureDeviceHostObject {
    /// `AVMediaType*` string this device claims to provide. Always
    /// `AVMediaTypeVideo` in our stub.
    media_type: id,
    /// Position (front/back). 1=back (default), 2=front.
    position: i32,
    /// Localized display name. Always "HyperHLE Stub Camera" in this stub.
    localized_name: id,
    /// Unique device id.
    unique_id: id,
}
impl HostObject for AVCaptureDeviceHostObject {}

#[derive(Default)]
struct AVCaptureDeviceInputHostObject {
    device: id,
}
impl HostObject for AVCaptureDeviceInputHostObject {}

#[derive(Default)]
struct AVCaptureVideoDataOutputHostObject {
    sample_buffer_delegate: id,
    sample_buffer_queue: id,
    video_settings: id,
    always_discards_late_video_frames: bool,
}
impl HostObject for AVCaptureVideoDataOutputHostObject {}

#[derive(Default)]
struct AVCaptureMetadataOutputHostObject {
    metadata_objects_delegate: id,
    metadata_objects_queue: id,
    metadata_object_types: id,
    rect_of_interest: CGRect,
}
impl HostObject for AVCaptureMetadataOutputHostObject {}

/// AVCaptureVideoPreviewLayer is a subclass of CALayer in iOS. We reuse
/// `CALayerHostObject` (allocated by the parent class's `+allocWithZone:`)
/// and store our extra state in a separate side-table indexed by the layer's
/// `id`. The side-table lives on
/// `Environment::framework_state::avfoundation::av_capture_preview_extras`.
#[derive(Default)]
pub struct AVCapturePreviewLayerExtra {
    pub(crate) session: id,
    pub(crate) video_gravity: id,
}

// ============================================================================
// MARK: - Class implementations
// ============================================================================

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// ----------------------------------------------------------------------------
// AVCaptureSession
// ----------------------------------------------------------------------------
@implementation AVCaptureSession: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<AVCaptureSessionHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init {
    let preset = ns_string::get_static_str(env, AVCaptureSessionPresetMedium);
    {
        let host = env.objc.borrow_mut::<AVCaptureSessionHostObject>(this);
        host.preset = preset;
    }
    log!("[(AVCaptureSession*){:?} init] (HyperHLE stub)", this);
    this
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<AVCaptureSessionHostObject>(this));
    for input in host.inputs {
        release(env, input);
    }
    for output in host.outputs {
        release(env, output);
    }
    env.framework_state.avfoundation.av_capture.running_sessions.retain(|&s| s != this);
    env.objc.dealloc_object(this, &mut env.mem);
}

// MARK: Configuration

- (())beginConfiguration {
    // No-op: we apply changes immediately.
}
- (())commitConfiguration {
    // No-op.
}

- (id)sessionPreset {
    env.objc.borrow::<AVCaptureSessionHostObject>(this).preset
}

- (())setSessionPreset:(id)preset {
    let old = env.objc.borrow::<AVCaptureSessionHostObject>(this).preset;
    retain(env, preset);
    release(env, old);
    env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).preset = preset;
}

- (bool)canSetSessionPreset:(id)_preset {
    // Accept anything; we don't actually drive a real camera here.
    true
}

// MARK: Inputs / outputs

- (id)inputs {
    let inputs: Vec<id> = env
        .objc
        .borrow::<AVCaptureSessionHostObject>(this)
        .inputs
        .clone();
    for &i in &inputs {
        retain(env, i);
    }
    let arr = ns_array::from_vec(env, inputs);
    autorelease(env, arr)
}

- (id)outputs {
    let outputs: Vec<id> = env
        .objc
        .borrow::<AVCaptureSessionHostObject>(this)
        .outputs
        .clone();
    for &o in &outputs {
        retain(env, o);
    }
    let arr = ns_array::from_vec(env, outputs);
    autorelease(env, arr)
}

- (())addInput:(id)input {
    if input == nil { return; }
    retain(env, input);
    env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).inputs.push(input);
}
- (bool)canAddInput:(id)_input { true }

- (())addOutput:(id)output {
    if output == nil { return; }
    retain(env, output);
    env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).outputs.push(output);
}
- (bool)canAddOutput:(id)_output { true }

- (())removeInput:(id)input {
    let pos = {
        let host = env.objc.borrow::<AVCaptureSessionHostObject>(this);
        host.inputs.iter().position(|&i| i == input)
    };
    if let Some(pos) = pos {
        env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).inputs.remove(pos);
        release(env, input);
    }
}

- (())removeOutput:(id)output {
    let pos = {
        let host = env.objc.borrow::<AVCaptureSessionHostObject>(this);
        host.outputs.iter().position(|&o| o == output)
    };
    if let Some(pos) = pos {
        env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).outputs.remove(pos);
        release(env, output);
    }
}

// MARK: Lifecycle

- (())startRunning {
    log!("[(AVCaptureSession*){:?} startRunning] (HyperHLE stub)", this);
    env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).running = true;
    if !env.framework_state.avfoundation.av_capture.running_sessions.contains(&this) {
        env.framework_state.avfoundation.av_capture.running_sessions.push(this);
    }
    // Notify observers via NSNotificationCenter.
    let nc: id = msg_class![env; NSNotificationCenter defaultCenter];
    let name = ns_string::get_static_str(env, AVCaptureSessionDidStartRunningNotification);
    () = msg![env; nc postNotificationName:name object:this];
}

- (())stopRunning {
    log!("[(AVCaptureSession*){:?} stopRunning] (HyperHLE stub)", this);
    env.objc.borrow_mut::<AVCaptureSessionHostObject>(this).running = false;
    env.framework_state.avfoundation.av_capture.running_sessions.retain(|&s| s != this);
    let nc: id = msg_class![env; NSNotificationCenter defaultCenter];
    let name = ns_string::get_static_str(env, AVCaptureSessionDidStopRunningNotification);
    () = msg![env; nc postNotificationName:name object:this];
}

- (bool)isRunning {
    env.objc.borrow::<AVCaptureSessionHostObject>(this).running
}
- (bool)isInterrupted {
    env.objc.borrow::<AVCaptureSessionHostObject>(this).interrupted
}

@end


// ----------------------------------------------------------------------------
// AVCaptureDevice
// ----------------------------------------------------------------------------
@implementation AVCaptureDevice: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<AVCaptureDeviceHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)defaultDeviceWithMediaType:(id)media_type {
    if media_type == nil {
        return nil;
    }
    let want_video = ns_string::get_static_str(env, AVMediaTypeVideo);
    let same: bool = msg![env; media_type isEqualToString:want_video];
    if !same {
        return nil;
    }
    make_default_video_device(env)
}

+ (id)devicesWithMediaType:(id)media_type {
    if media_type == nil {
        return msg_class![env; NSArray array];
    }
    let want_video = ns_string::get_static_str(env, AVMediaTypeVideo);
    let same: bool = msg![env; media_type isEqualToString:want_video];
    if !same {
        return msg_class![env; NSArray array];
    }
    let dev = make_default_video_device(env);
    let arr: id = msg_class![env; NSArray arrayWithObject:dev];
    arr
}

+ (id)devices {
    let dev = make_default_video_device(env);
    let arr: id = msg_class![env; NSArray arrayWithObject:dev];
    arr
}

- (id)mediaType {
    env.objc.borrow::<AVCaptureDeviceHostObject>(this).media_type
}
- (bool)hasMediaType:(id)mt {
    let m = env.objc.borrow::<AVCaptureDeviceHostObject>(this).media_type;
    if m == nil || mt == nil { return false; }
    msg![env; m isEqualToString:mt]
}

// AVCaptureDevicePosition: 0=Unspecified, 1=Back, 2=Front.
- (i32)position {
    env.objc.borrow::<AVCaptureDeviceHostObject>(this).position
}

- (id)localizedName {
    env.objc.borrow::<AVCaptureDeviceHostObject>(this).localized_name
}
- (id)uniqueID {
    env.objc.borrow::<AVCaptureDeviceHostObject>(this).unique_id
}

- (bool)lockForConfiguration:(crate::mem::MutPtr<id>)_error { true }
- (())unlockForConfiguration {}

// Most boolean queries the app might ask: just say "no, not supported".
- (bool)hasFlash { false }
- (bool)hasTorch { false }
- (bool)isFocusModeSupported:(i32)_mode { false }
- (bool)isExposureModeSupported:(i32)_mode { false }
- (bool)isFlashModeSupported:(i32)_mode { false }
- (bool)isTorchModeSupported:(i32)_mode { false }
- (bool)isWhiteBalanceModeSupported:(i32)_mode { false }

- (bool)isFocusPointOfInterestSupported { false }
- (bool)isExposurePointOfInterestSupported { false }

@end


// ----------------------------------------------------------------------------
// AVCaptureDeviceInput
// ----------------------------------------------------------------------------
@implementation AVCaptureDeviceInput: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<AVCaptureDeviceInputHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)deviceInputWithDevice:(id)device error:(crate::mem::MutPtr<id>)error {
    if device == nil {
        if !error.is_null() {
            env.mem.write(error, nil);
        }
        return nil;
    }
    let cls: crate::objc::Class = env.objc.get_known_class("AVCaptureDeviceInput", &mut env.mem);
    let alloc: id = msg![env; cls alloc];
    let initialized: id = msg![env; alloc initWithDevice:device error:error];
    autorelease(env, initialized)
}

- (id)initWithDevice:(id)device error:(crate::mem::MutPtr<id>)error {
    retain(env, device);
    env.objc.borrow_mut::<AVCaptureDeviceInputHostObject>(this).device = device;
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    this
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<AVCaptureDeviceInputHostObject>(this));
    if host.device != nil {
        release(env, host.device);
    }
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)device {
    env.objc.borrow::<AVCaptureDeviceInputHostObject>(this).device
}

@end


// ----------------------------------------------------------------------------
// AVCaptureOutput (base class — apps may message it generically)
// ----------------------------------------------------------------------------
@implementation AVCaptureOutput: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    // Plain NSObject HostObject — subclasses override.
    let host: Box<TrivialHostObject> = Box::new(TrivialHostObject);
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)connections { msg_class![env; NSArray array] }
- (id)connectionWithMediaType:(id)_media_type { nil }

@end


// ----------------------------------------------------------------------------
// AVCaptureVideoDataOutput
// ----------------------------------------------------------------------------
@implementation AVCaptureVideoDataOutput: AVCaptureOutput

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<AVCaptureVideoDataOutputHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init {
    log!("[(AVCaptureVideoDataOutput*){:?} init] (HyperHLE stub)", this);
    this
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<AVCaptureVideoDataOutputHostObject>(this));
    if host.video_settings != nil { release(env, host.video_settings); }
    if host.sample_buffer_queue != nil { release(env, host.sample_buffer_queue); }
    env.objc.dealloc_object(this, &mut env.mem);
}

- (())setSampleBufferDelegate:(id)delegate queue:(id)queue {
    // Delegates are weak in Apple's API, so we don't retain `delegate`.
    if queue != nil { retain(env, queue); }
    let old_q = env.objc.borrow::<AVCaptureVideoDataOutputHostObject>(this).sample_buffer_queue;
    if old_q != nil { release(env, old_q); }
    let host = env.objc.borrow_mut::<AVCaptureVideoDataOutputHostObject>(this);
    host.sample_buffer_delegate = delegate;
    host.sample_buffer_queue = queue;
}

- (id)sampleBufferDelegate {
    env.objc.borrow::<AVCaptureVideoDataOutputHostObject>(this).sample_buffer_delegate
}
- (id)sampleBufferCallbackQueue {
    env.objc.borrow::<AVCaptureVideoDataOutputHostObject>(this).sample_buffer_queue
}

- (id)videoSettings {
    env.objc.borrow::<AVCaptureVideoDataOutputHostObject>(this).video_settings
}

- (())setVideoSettings:(id)settings {
    let old = env.objc.borrow::<AVCaptureVideoDataOutputHostObject>(this).video_settings;
    let copy: id = if settings != nil { msg![env; settings copy] } else { nil };
    release(env, old);
    env.objc.borrow_mut::<AVCaptureVideoDataOutputHostObject>(this).video_settings = copy;
}

- (())setAlwaysDiscardsLateVideoFrames:(bool)flag {
    env.objc
        .borrow_mut::<AVCaptureVideoDataOutputHostObject>(this)
        .always_discards_late_video_frames = flag;
}
- (bool)alwaysDiscardsLateVideoFrames {
    env.objc
        .borrow::<AVCaptureVideoDataOutputHostObject>(this)
        .always_discards_late_video_frames
}

@end


// ----------------------------------------------------------------------------
// AVCaptureMetadataOutput (barcode detection)
// ----------------------------------------------------------------------------
@implementation AVCaptureMetadataOutput: AVCaptureOutput

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<AVCaptureMetadataOutputHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init {
    log!("[(AVCaptureMetadataOutput*){:?} init] (HyperHLE stub)", this);
    let host = env.objc.borrow_mut::<AVCaptureMetadataOutputHostObject>(this);
    host.rect_of_interest = CGRect {
        origin: crate::frameworks::core_graphics::CGPoint { x: 0.0, y: 0.0 },
        size: crate::frameworks::core_graphics::CGSize { width: 1.0, height: 1.0 },
    };
    this
}

- (())dealloc {
    let host = std::mem::take(env.objc.borrow_mut::<AVCaptureMetadataOutputHostObject>(this));
    if host.metadata_object_types != nil { release(env, host.metadata_object_types); }
    if host.metadata_objects_queue != nil { release(env, host.metadata_objects_queue); }
    env.objc.dealloc_object(this, &mut env.mem);
}

- (())setMetadataObjectsDelegate:(id)delegate queue:(id)queue {
    if queue != nil { retain(env, queue); }
    let old_q = env.objc.borrow::<AVCaptureMetadataOutputHostObject>(this).metadata_objects_queue;
    if old_q != nil { release(env, old_q); }
    let host = env.objc.borrow_mut::<AVCaptureMetadataOutputHostObject>(this);
    host.metadata_objects_delegate = delegate;
    host.metadata_objects_queue = queue;
}

- (id)availableMetadataObjectTypes {
    let types: Vec<id> = [
        AVMetadataObjectTypeQRCode,
        AVMetadataObjectTypeEAN13Code,
        AVMetadataObjectTypeEAN8Code,
        AVMetadataObjectTypeUPCECode,
        AVMetadataObjectTypeCode128Code,
        AVMetadataObjectTypeCode39Code,
        AVMetadataObjectTypeCode93Code,
        AVMetadataObjectTypePDF417Code,
        AVMetadataObjectTypeAztecCode,
        AVMetadataObjectTypeFace,
    ]
    .iter()
    .map(|s| {
        let str_id = ns_string::get_static_str(env, s);
        retain(env, str_id);
        str_id
    })
    .collect();
    let arr = ns_array::from_vec(env, types);
    autorelease(env, arr)
}

- (id)metadataObjectTypes {
    env.objc.borrow::<AVCaptureMetadataOutputHostObject>(this).metadata_object_types
}
- (())setMetadataObjectTypes:(id)types {
    let old = env.objc.borrow::<AVCaptureMetadataOutputHostObject>(this).metadata_object_types;
    let copy: id = if types != nil { msg![env; types copy] } else { nil };
    release(env, old);
    env.objc.borrow_mut::<AVCaptureMetadataOutputHostObject>(this).metadata_object_types = copy;
}

- (CGRect)rectOfInterest {
    env.objc.borrow::<AVCaptureMetadataOutputHostObject>(this).rect_of_interest
}
- (())setRectOfInterest:(CGRect)rect {
    env.objc.borrow_mut::<AVCaptureMetadataOutputHostObject>(this).rect_of_interest = rect;
}

@end


// ----------------------------------------------------------------------------
// AVCaptureVideoPreviewLayer  — extends CALayer
// ----------------------------------------------------------------------------
//
// AVCaptureVideoPreviewLayer is a CALayer subclass. The base CALayer has all
// the geometry / hierarchy machinery. We add session/gravity tracking via a
// side-table because the base class's `+allocWithZone:` already allocates
// a `CALayerHostObject`.
@implementation AVCaptureVideoPreviewLayer: CALayer

+ (id)layerWithSession:(id)session {
    let cls: crate::objc::Class =
        env.objc.get_known_class("AVCaptureVideoPreviewLayer", &mut env.mem);
    let alloc: id = msg![env; cls alloc];
    let initialized: id = msg![env; alloc initWithSession:session];
    autorelease(env, initialized)
}

- (id)initWithSession:(id)session {
    let _: id = msg_super![env; this init];
    set_preview_layer_session(env, this, session);
    let gravity = ns_string::get_static_str(env, AVLayerVideoGravityResizeAspect);
    set_preview_layer_gravity(env, this, gravity);

    // Make the preview layer visibly grey so apps that put their UI on top
    // of it don't render a black screen even before any frames arrive.
    // 50 % grey makes it obvious the stub is active.
    let cg_color: id = msg_class![env; UIColor darkGrayColor];
    let cg: id = msg![env; cg_color CGColor];
    () = msg![env; this setBackgroundColor:cg];
    this
}

- (id)session {
    preview_layer_session(env, this)
}
- (())setSession:(id)session {
    set_preview_layer_session(env, this, session);
}

- (id)videoGravity {
    preview_layer_gravity(env, this)
}
- (())setVideoGravity:(id)gravity {
    set_preview_layer_gravity(env, this, gravity);
}

- (bool)isOrientationSupported { true }
- (i32)orientation { 1 /* AVCaptureVideoOrientationPortrait */ }
- (())setOrientation:(i32)_orientation {}

@end

};

// ============================================================================
// MARK: - Helpers
// ============================================================================

fn make_default_video_device(env: &mut crate::Environment) -> id {
    let cls: crate::objc::Class = env.objc.get_known_class("AVCaptureDevice", &mut env.mem);
    let alloc: id = msg![env; cls alloc];
    let init: id = msg![env; alloc init];
    let media_type = ns_string::get_static_str(env, AVMediaTypeVideo);
    retain(env, media_type);
    let name = ns_string::from_rust_string(env, "HyperHLE Stub Camera".to_string());
    let uid = ns_string::from_rust_string(env, "com.hyperhle.camera.stub".to_string());
    {
        let host = env.objc.borrow_mut::<AVCaptureDeviceHostObject>(init);
        host.media_type = media_type;
        host.position = 1; // Back camera by default.
        host.localized_name = name;
        host.unique_id = uid;
    }
    autorelease(env, init)
}

// AVCaptureVideoPreviewLayer side-table.
//
// Stored on the framework state because the base CALayer host object can't
// be subclassed after the fact.
use std::collections::HashMap;
fn preview_layer_extras(
    env: &mut crate::Environment,
) -> &mut HashMap<id, AVCapturePreviewLayerExtra> {
    &mut env.framework_state.avfoundation.av_capture_preview_extras
}

fn preview_layer_session(env: &mut crate::Environment, layer: id) -> id {
    preview_layer_extras(env)
        .get(&layer)
        .map(|e| e.session)
        .unwrap_or(nil)
}

fn preview_layer_gravity(env: &mut crate::Environment, layer: id) -> id {
    preview_layer_extras(env)
        .get(&layer)
        .map(|e| e.video_gravity)
        .unwrap_or(nil)
}

fn set_preview_layer_session(env: &mut crate::Environment, layer: id, session: id) {
    let old = preview_layer_extras(env)
        .get(&layer)
        .map(|e| e.session)
        .unwrap_or(nil);
    if session != nil {
        retain(env, session);
    }
    if old != nil {
        release(env, old);
    }
    preview_layer_extras(env).entry(layer).or_default().session = session;
}

fn set_preview_layer_gravity(env: &mut crate::Environment, layer: id, gravity: id) {
    let old = preview_layer_extras(env)
        .get(&layer)
        .map(|e| e.video_gravity)
        .unwrap_or(nil);
    if gravity != nil {
        retain(env, gravity);
    }
    if old != nil {
        release(env, old);
    }
    preview_layer_extras(env)
        .entry(layer)
        .or_default()
        .video_gravity = gravity;
}
