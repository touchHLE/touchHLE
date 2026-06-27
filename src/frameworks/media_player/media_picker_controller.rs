/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaPickerController` and `MPMediaPickerControllerDelegate`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// MARK: - MPMediaType bitmask constants (used by allowedMediaTypes)
pub type MPMediaType = u32;
pub const MP_MEDIA_TYPE_MUSIC: MPMediaType = 1 << 0;
pub const MP_MEDIA_TYPE_PODCAST: MPMediaType = 1 << 1;
pub const MP_MEDIA_TYPE_AUDIO_BOOK: MPMediaType = 1 << 2;
pub const MP_MEDIA_TYPE_ANY_AUDIO: MPMediaType = 0x00ff;
pub const MP_MEDIA_TYPE_MOVIE: MPMediaType = 1 << 8;
pub const MP_MEDIA_TYPE_TV_SHOW: MPMediaType = 1 << 9;
pub const MP_MEDIA_TYPE_VIDEO_PODCAST: MPMediaType = 1 << 10;
pub const MP_MEDIA_TYPE_MUSIC_VIDEO: MPMediaType = 1 << 11;
pub const MP_MEDIA_TYPE_VIDEO_ITunes: MPMediaType = 1 << 12;
pub const MP_MEDIA_TYPE_ANY_VIDEO: MPMediaType = 0xff00;
pub const MP_MEDIA_TYPE_ANY: MPMediaType = !0;

#[derive(Default)]
struct MPMediaPickerControllerHostObject {
    /// `MPMediaType` bitmask
    allowed_media_types: MPMediaType,
    /// Whether the user can select multiple items
    allows_picking_multiple_items: bool,
    /// Prompt NSString* shown above the item list
    prompt: id,
    /// Delegate id (weak reference)
    delegate: id,
    /// Whether to show cloud items
    show_network_activity: bool,
}
impl HostObject for MPMediaPickerControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaPickerController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MPMediaPickerControllerHostObject {
        allowed_media_types: MP_MEDIA_TYPE_ANY,
        allows_picking_multiple_items: false,
        prompt: nil,
        delegate: nil,
        show_network_activity: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (id)initWithMediaTypes:(MPMediaType)media_types {
    env.objc.borrow_mut::<MPMediaPickerControllerHostObject>(this)
        .allowed_media_types = media_types;
    this
}

- (())dealloc {
    let host = env.objc.borrow::<MPMediaPickerControllerHostObject>(this);
    let (prompt, delegate) = (host.prompt, host.delegate);
    release(env, prompt);
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Media types

- (MPMediaType)allowedMediaTypes {
    env.objc.borrow::<MPMediaPickerControllerHostObject>(this).allowed_media_types
}

- (())setAllowedMediaTypes:(MPMediaType)media_types {
    env.objc.borrow_mut::<MPMediaPickerControllerHostObject>(this)
        .allowed_media_types = media_types;
}

// MARK: - Multiple selection

- (bool)allowsPickingMultipleItems {
    env.objc.borrow::<MPMediaPickerControllerHostObject>(this)
        .allows_picking_multiple_items
}

- (())setAllowsPickingMultipleItems:(bool)allows {
    env.objc.borrow_mut::<MPMediaPickerControllerHostObject>(this)
        .allows_picking_multiple_items = allows;
}

// MARK: - Prompt

- (id)prompt { // NSString*
    env.objc.borrow::<MPMediaPickerControllerHostObject>(this).prompt
}

- (())setPrompt:(id)prompt { // NSString*
    let old = env.objc.borrow::<MPMediaPickerControllerHostObject>(this).prompt;
    release(env, old);
    retain(env, prompt);
    env.objc.borrow_mut::<MPMediaPickerControllerHostObject>(this).prompt = prompt;
}

// MARK: - Network / cloud items

- (bool)showsCloudItems {
    env.objc.borrow::<MPMediaPickerControllerHostObject>(this).show_network_activity
}

- (())setShowsCloudItems:(bool)shows {
    env.objc.borrow_mut::<MPMediaPickerControllerHostObject>(this)
        .show_network_activity = shows;
}

// MARK: - Delegate

- (id)delegate {
    env.objc.borrow::<MPMediaPickerControllerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<MPMediaPickerControllerHostObject>(this).delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<MPMediaPickerControllerHostObject>(this).delegate = delegate;
}

// MARK: - UIViewController overrides
// The picker has no real UI in touchHLE. When presented we immediately notify
// the delegate that the user cancelled, which is the safest no-op behaviour.

- (())viewDidLoad {
    log!("MPMediaPickerController: viewDidLoad — no media library available");
}

- (())viewWillAppear:(bool)_animated {
    // No super call needed — UIViewController's default is a no-op.

    let delegate = env.objc.borrow::<MPMediaPickerControllerHostObject>(this).delegate;
    if delegate != nil {
        let _: () = msg![env; delegate mediaPickerDidCancel:this];
    }
}

- (())viewDidAppear:(bool)animated {
    // No super call needed — UIViewController's default is a no-op.

    let presenting: id = msg![env; this presentingViewController];
    if presenting != nil {
        let _: () = msg![env; presenting dismissViewControllerAnimated:animated completion:nil];
    }
}

// MARK: - Description

- (id)description {
    let (allowed_media_types, allows_picking_multiple_items, prompt) = {
        let host = env.objc.borrow::<MPMediaPickerControllerHostObject>(this);
        (host.allowed_media_types, host.allows_picking_multiple_items, host.prompt)
    }; // immutable borrow ends here

    let prompt_str = if prompt != nil {
        ns_string::to_rust_string(env, prompt).into_owned()
    } else {
        "(null)".to_string()
    };
    let s = format!(
        "<MPMediaPickerController: mediaTypes={:#x} multipleItems={} prompt={}>",
        allowed_media_types, allows_picking_multiple_items, prompt_str
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    let desc: id = msg_class![env; NSString stringWithUTF8String:cstr];
    desc
}

@end

};
