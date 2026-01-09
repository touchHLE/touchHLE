/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWindow`.
//!
//! Useful resources:
//! - [Technical Q&A QA1588: Automatic orientation support for iPhone and iPad apps](https://developer.apple.com/library/archive/qa/qa1588/_index.html)
//! - [Technical Q&A QA1688: Why won't my UIViewController rotate with the device?](https://developer.apple.com/library/archive/qa/qa1688/_index.html)

use super::UIViewHostObject;
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::cg_affine_transform::CGAffineTransform;
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_application::{
    UIInterfaceOrientationLandscapeLeft, UIInterfaceOrientationLandscapeRight,
};
use crate::frameworks::uikit::ui_device::{
    UIDeviceOrientationLandscapeLeft, UIDeviceOrientationLandscapeRight,
};
use crate::objc::{id, msg, msg_class, msg_super, nil, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    /// List of visible windows for internal purposes. Non-retaining!
    ///
    /// This is public because Core Animation also uses it.
    pub windows: Vec<id>,
    /// The most recent window which received `makeKeyAndVisible` message.
    /// Non-retaining!
    pub key_window: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// TODO: more?

- (id)initWithFrame:(CGRect)frame {
    let this = msg_super![env; this initWithFrame:frame];
    // Undocumented: windows seem to be hidden by default on iOS, unlike views.
    // Super call to bypass the overriden setter on this class, which would post
    // a notification.
    () = msg_super![env; this setHidden:true];

    let list = &mut env.framework_state.uikit.ui_view.ui_window.windows;
    list.push(this);
    log_dbg!(
        "New window: {:?}. New list of all windows: {:?}",
        this,
        list,
    );

    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this = msg_super![env; this initWithCoder:coder];
    // Undocumented: windows seem to be hidden by default on iOS, unlike views.
    // Super call to bypass the overriden setter on this class, which would post
    // a notification.
    () = msg_super![env; this setHidden:true];

    let list = &mut env.framework_state.uikit.ui_view.ui_window.windows;
    list.push(this);
    log_dbg!(
        "New window: {:?}. New list of all windows: {:?}",
        this,
        list,
    );

    this
}

- (())dealloc {
    if let Some(key_window) = env.framework_state.uikit.ui_view.ui_window.key_window {
        if key_window == this {
            env.framework_state.uikit.ui_view.ui_window.key_window = None;
        }
    }
    let list = &mut env.framework_state.uikit.ui_view.ui_window.windows;
    let idx = list.iter().position(|&w| w == this).unwrap();
    list.remove(idx);
    log_dbg!(
        "Deallocating window {:?}. New list of all windows: {:?}",
        this,
        list,
    );
    msg_super![env; this dealloc]
}

- (())setHidden:(bool)is_hidden {
    () = msg_super![env; this setHidden:is_hidden];

    // TODO: post UIWindowDidBecomeVisibleNotification,
    //            UIWindowDidBecomeHiddenNotification
    log_dbg!("[(UIWindow*){:?} setHidden:{:?}]", this, is_hidden);
}

- (())makeKeyWindow {
    // TODO: post UIWindowDidResignKeyNotification for previous key window
    env.framework_state.uikit.ui_view.ui_window.key_window = Some(this);

    let center: id = msg_class![env; NSNotificationCenter defaultCenter];
    let notif_name = ns_string::get_static_str(env, UIWindowDidBecomeKeyNotification);
    () = msg![env; center postNotificationName:notif_name object:this userInfo:nil];
}

- (())makeKeyAndVisible {
    // TODO: We don't currently have send any non-touch events to windows,
    // so there's no meaning in it yet.

    // FIXME: This should also bump the window to the top of the list.

    () = msg![env; this makeKeyWindow];

    // TODO: post UIWindowDidBecomeVisibleNotification
    () = msg![env; this setHidden:false];
}

// UIResponder implementation
// From the Apple UIView docs regarding [UIResponder nextResponder]:
// "UIWindow returns the application object."
- (id)nextResponder {
    msg_class![env; UIApplication sharedApplication]
}

- (())addSubview:(id)view {
    log_dbg!("[(UIWindow*){:?} addSubview:{:?}] => ()", this, view);

    if view == nil || env.objc.borrow::<UIViewHostObject>(view).view_controller == nil {
        () = msg_super![env; this addSubview:view];
        return;
    }

    // Below we treat a special case of adding view controller's view
    // to a window, in order to generate display related notifications

    if env.objc.borrow::<UIViewHostObject>(this).subviews.contains(&view) {
        // For the case of existing view hidden by another view,
        // we need to delay a below sequence up until obstructions are removed
        log!("TODO: case of existing view hidden by another view for sending view[Will,Did]Appear");
    }

    let vc = env.objc.borrow::<UIViewHostObject>(view).view_controller;
    () = msg![env; vc viewWillAppear:false];
    () = msg_super![env; this addSubview:view];
    () = msg![env; vc viewDidAppear:false];

    // Support auto-rotation. This is currently only for apps that request a
    // non-portrait interface orientation via Info.plist, as we do not yet
    // support changes of orientation caused by device rotation (TODO).
    // FIXME: It's unclear when and where this auto-rotation is supposed to
    //        happen. It must have something to do with mounting the view
    //        controller to a window, so we do it here. QA1688 (see top of file)
    //        mentions a breaking behaviour change in iOS 6 that makes
    //        auto-rotation rely on rootViewController (a property only found in
    //        iOS 6), so the current implementation is specific to iOS <= 5.
    // FIXME: Are we supposed to notify the view somehow of the rotation?
    // FIXME: What do we do if shouldAutorotateToInterfaceOrientation:
    //        returns false? The status bar has already been rotated…
    // FIXME: The device orientation stored on env.window can come from one of
    //        three places (user/default options, setStatusBarOrientation: etc,
    //        Info.plist UIInterfaceOrientation etc). It's not clear if these
    //        are really equivalent and should all trigger autorotation.
    if let Some(orientation) = match env.window.as_ref().unwrap().current_rotation() {
        crate::window::DeviceOrientation::LandscapeLeft => Some(UIDeviceOrientationLandscapeLeft),
        crate::window::DeviceOrientation::LandscapeRight => Some(UIDeviceOrientationLandscapeRight),
        // Portrait is the default so we don't do anything here.
        crate::window::DeviceOrientation::Portrait => None,
    } {
        // (UIInterfaceOrientation and UIDeviceOrientation are compatible enums,
        //  here we use whichever is clearer contextually.)
        let should = msg![env; vc shouldAutorotateToInterfaceOrientation:orientation];
        log_dbg!("[{:?} shouldAutorotateToInterfaceOrientation:{:?}] => {:?}", vc, orientation, should);
        if should {
            log_dbg!("App requested autorotation; applying orientation transform to view {:?}.", view);
            let transform = match orientation {
                UIInterfaceOrientationLandscapeLeft => CGAffineTransform::make_rotation(-std::f32::consts::FRAC_PI_2),
                UIInterfaceOrientationLandscapeRight => CGAffineTransform::make_rotation(std::f32::consts::FRAC_PI_2),
                _ => unimplemented!(),
            };

            let window_frame: CGRect = msg![env; this frame];
            log_dbg!("Window frame: {window_frame:?}");
            let view_frame: CGRect = msg![env; view frame];
            log_dbg!("Old view frame: {view_frame:?}");

            () = msg![env; view setTransform:transform];

            // Re-apply the view's old frame to compensate for the rotation
            // effectively offseting its center position and changing the size.
            // FIXME: I have no idea if this is how this should be solved, but
            //        it works for DMC4 Refrain at least.

            let view_frame: CGRect = msg![env; view frame];
            log_dbg!("Old view frame after transform: {view_frame:?}");

            () = msg![env; view setFrame:window_frame];

            let view_frame: CGRect = msg![env; view frame];
            log_dbg!("New view frame after re-applying old view frame: {view_frame:?}");
        }
    }
}

- (CGPoint)convertPoint:(CGPoint)point
             fromWindow:(id)other { // UIWindow*
    let this_layer: id = msg![env; this layer];
    // Resolves to nil if other is nil.
    let other_layer: id = msg![env; other layer];
    msg![env; this_layer convertPoint:point fromLayer:other_layer]
}
- (CGPoint)convertPoint:(CGPoint)point
               toWindow:(id)other { // UIWindow*
    let this_layer: id = msg![env; this layer];
    // Resolves to nil if other is nil.
    let other_layer: id = msg![env; other layer];
    msg![env; this_layer convertPoint:point toLayer:other_layer]
}

@end

};

/// Window life-cycle notifications
/// TODO: more notifications
const UIWindowDidBecomeKeyNotification: &str = "UIWindowDidBecomeKeyNotification";
/// Keyboard notifications
/// TODO: more keyboard notifications
pub const UIKeyboardWillShowNotification: &str = "UIKeyboardWillShowNotification";
pub const UIKeyboardDidShowNotification: &str = "UIKeyboardDidShowNotification";
pub const UIKeyboardWillHideNotification: &str = "UIKeyboardWillHideNotification";
pub const UIKeyboardDidHideNotification: &str = "UIKeyboardDidHideNotification";
pub const UIKeyboardBoundsUserInfoKey: &str = "UIKeyboardBoundsUserInfoKey";

pub const CONSTANTS: ConstantExports = &[
    (
        "_UIWindowDidBecomeKeyNotification",
        HostConstant::NSString(UIWindowDidBecomeKeyNotification),
    ),
    (
        "_UIKeyboardWillShowNotification",
        HostConstant::NSString(UIKeyboardWillShowNotification),
    ),
    (
        "_UIKeyboardDidShowNotification",
        HostConstant::NSString(UIKeyboardDidShowNotification),
    ),
    (
        "_UIKeyboardWillHideNotification",
        HostConstant::NSString(UIKeyboardWillHideNotification),
    ),
    (
        "_UIKeyboardDidHideNotification",
        HostConstant::NSString(UIKeyboardDidHideNotification),
    ),
    (
        "_UIKeyboardBoundsUserInfoKey",
        HostConstant::NSString(UIKeyboardBoundsUserInfoKey),
    ),
];
