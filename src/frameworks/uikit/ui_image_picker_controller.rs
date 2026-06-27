/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `UIImagePickerController`

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;

type UIImagePickerControllerSourceType = NSInteger;
type UIImagePickerControllerQualityType = NSInteger;
type UIImagePickerControllerCameraCaptureMode = NSInteger;
type UIImagePickerControllerCameraDevice = NSInteger;
type UIImagePickerControllerCameraFlashMode = NSInteger;

// UIImagePickerControllerSourceType values
const UIImagePickerControllerSourceTypePhotoLibrary: NSInteger = 0;
const UIImagePickerControllerSourceTypeCamera: NSInteger = 1;
const UIImagePickerControllerSourceTypeSavedPhotosAlbum: NSInteger = 2;

// UIImagePickerControllerQualityType values
const UIImagePickerControllerQualityTypeHigh: NSInteger = 0;
const UIImagePickerControllerQualityTypeMedium: NSInteger = 1;
const UIImagePickerControllerQualityTypeLow: NSInteger = 2;

// UIImagePickerControllerCameraDevice values
const UIImagePickerControllerCameraDeviceRear: NSInteger = 0;
const UIImagePickerControllerCameraDeviceFront: NSInteger = 1;

// UIImagePickerControllerCameraFlashMode values
const UIImagePickerControllerCameraFlashModeOff: NSInteger = -1;
const UIImagePickerControllerCameraFlashModeAuto: NSInteger = 0;
const UIImagePickerControllerCameraFlashModeOn: NSInteger = 1;

#[derive(Default)]
struct UIImagePickerControllerHostObject {
    delegate: id,
    source_type: UIImagePickerControllerSourceType,
    media_types: id, // NSArray* of NSString*
    allows_editing: bool,
    video_quality: UIImagePickerControllerQualityType,
    camera_device: UIImagePickerControllerCameraDevice,
    camera_flash_mode: UIImagePickerControllerCameraFlashMode,
    camera_capture_mode: UIImagePickerControllerCameraCaptureMode,
    shows_camera_controls: bool,
    camera_overlay_view: id,
}
impl HostObject for UIImagePickerControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImagePickerController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIImagePickerControllerHostObject {
        delegate: nil,
        source_type: UIImagePickerControllerSourceTypePhotoLibrary,
        media_types: nil,
        allows_editing: false,
        video_quality: UIImagePickerControllerQualityTypeMedium,
        camera_device: UIImagePickerControllerCameraDeviceRear,
        camera_flash_mode: UIImagePickerControllerCameraFlashModeAuto,
        camera_capture_mode: 0,
        shows_camera_controls: true,
        camera_overlay_view: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Source type

+ (bool)isSourceTypeAvailable:(UIImagePickerControllerSourceType)_source_type {
    false
}

+ (id)availableMediaTypesForSourceType:(UIImagePickerControllerSourceType)_source_type {
    // Return an empty array — no sources are available.
    msg_class![env; NSArray new]
}

+ (bool)isCameraDeviceAvailable:(UIImagePickerControllerCameraDevice)_device {
    false
}

+ (bool)isFlashAvailableForCameraDevice:(UIImagePickerControllerCameraDevice)_device {
    false
}

+ (id)availableCaptureModesForCameraDevice:(UIImagePickerControllerCameraDevice)_device {
    msg_class![env; NSArray new]
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<UIImagePickerControllerHostObject>(this);

    let (delegate, media_types, camera_overlay_view) =
        (host.delegate, host.media_types, host.camera_overlay_view);

    release(env, delegate);
    release(env, media_types);
    release(env, camera_overlay_view);

    env.objc.dealloc_object(this, &mut env.mem)
}

- (UIImagePickerControllerSourceType)sourceType {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).source_type
}

- (())setSourceType:(UIImagePickerControllerSourceType)source_type {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).source_type = source_type;
}

// MARK: - Delegate

- (id)delegate {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<UIImagePickerControllerHostObject>(this).delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).delegate = delegate;
}

// MARK: - Media types

- (id)mediaTypes {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).media_types
}

- (())setMediaTypes:(id)media_types {
    let old = env.objc.borrow::<UIImagePickerControllerHostObject>(this).media_types;
    release(env, old);
    retain(env, media_types);
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).media_types = media_types;
}

// MARK: - Editing

- (bool)allowsEditing {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).allows_editing
}

- (())setAllowsEditing:(bool)allows {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).allows_editing = allows;
}

// Deprecated alias.
- (bool)allowsImageEditing {
    msg![env; this allowsEditing]
}
- (())setAllowsImageEditing:(bool)allows {
    msg![env; this setAllowsEditing:allows]
}

// MARK: - Video quality

- (UIImagePickerControllerQualityType)videoQuality {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).video_quality
}

- (())setVideoQuality:(UIImagePickerControllerQualityType)quality {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).video_quality = quality;
}

// MARK: - Camera device & flash

- (UIImagePickerControllerCameraDevice)cameraDevice {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).camera_device
}

- (())setCameraDevice:(UIImagePickerControllerCameraDevice)device {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).camera_device = device;
}

- (UIImagePickerControllerCameraFlashMode)cameraFlashMode {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).camera_flash_mode
}

- (())setCameraFlashMode:(UIImagePickerControllerCameraFlashMode)mode {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).camera_flash_mode = mode;
}

- (UIImagePickerControllerCameraCaptureMode)cameraCaptureMode {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).camera_capture_mode
}

- (())setCameraCaptureMode:(UIImagePickerControllerCameraCaptureMode)mode {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).camera_capture_mode = mode;
}

// MARK: - Camera controls & overlay

- (bool)showsCameraControls {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).shows_camera_controls
}

- (())setShowsCameraControls:(bool)shows {
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).shows_camera_controls = shows;
}

- (id)cameraOverlayView {
    env.objc.borrow::<UIImagePickerControllerHostObject>(this).camera_overlay_view
}

- (())setCameraOverlayView:(id)view {
    let old = env.objc.borrow::<UIImagePickerControllerHostObject>(this).camera_overlay_view;
    release(env, old);
    retain(env, view);
    env.objc.borrow_mut::<UIImagePickerControllerHostObject>(this).camera_overlay_view = view;
}

// MARK: - Camera actions (stubs)

- (bool)startVideoCapture {
    log!("UIImagePickerController startVideoCapture: stubbed, returning false");
    false
}

- (())stopVideoCapture {
    log!("UIImagePickerController stopVideoCapture: stubbed");
}

- (())takePicture {
    log!("UIImagePickerController takePicture: stubbed");
}

// MARK: - Presentation (stubs)

- (())presentModalViewController:(id)_vc animated:(bool)_animated {
    log!("UIImagePickerController presentModalViewController: stubbed (picker never shown)");
}

- (())dismissModalViewControllerAnimated:(bool)_animated {
    log!("UIImagePickerController dismissModalViewControllerAnimated: stubbed");
}

@end

};

/// `UIVideoAtPathIsCompatibleWithSavedPhotosAlbum(NSString*)` — Apple's API
/// to ask whether a movie file can be saved to the Camera Roll. touchHLE
/// has no Photos library, so the honest answer is "no". Returning `false`
/// also matches the documented behaviour for files with unsupported
/// codecs/containers.
fn UIVideoAtPathIsCompatibleWithSavedPhotosAlbum(env: &mut Environment, video_path: id) -> bool {
    let path_str = if video_path == nil {
        "(nil)".to_string()
    } else {
        to_rust_string(env, video_path).into_owned()
    };
    log!(
        "UIVideoAtPathIsCompatibleWithSavedPhotosAlbum({:?}): stubbed, returning false",
        path_str
    );
    false
}

/// `UISaveVideoAtPathToSavedPhotosAlbum(NSString*, id, SEL, void*)` — Apple's
/// API to copy a movie to the Camera Roll. touchHLE has no Photos library
/// so we simply log and do nothing; the optional completion selector is
/// not invoked because the documented contract on real iOS is that the
/// callback signals "saved", not "ignored", and we'd rather have a quiet
/// no-op than spoof a false success.
fn UISaveVideoAtPathToSavedPhotosAlbum(
    env: &mut Environment,
    video_path: id,
    _completion_target: id,
    _completion_selector: crate::objc::SEL,
    _context_info: crate::mem::MutVoidPtr,
) {
    let path_str = if video_path == nil {
        "(nil)".to_string()
    } else {
        to_rust_string(env, video_path).into_owned()
    };
    log!(
        "UISaveVideoAtPathToSavedPhotosAlbum({:?}): stubbed (no Photos library in touchHLE)",
        path_str
    );
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(UIVideoAtPathIsCompatibleWithSavedPhotosAlbum(_)),
    export_c_func!(UISaveVideoAtPathToSavedPhotosAlbum(_, _, _, _)),
];
