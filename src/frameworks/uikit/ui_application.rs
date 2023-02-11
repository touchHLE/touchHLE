/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIApplication` and `UIApplicationMain`.

use super::ui_device::*;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_nib::load_main_nib_file;
use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::{id, msg, msg_class, nil, objc_classes, retain, ClassExports, HostObject};
use crate::window::DeviceOrientation;
use crate::Environment;

#[derive(Default)]
pub struct State {
    /// [UIApplication sharedApplication]
    shared_application: Option<id>,
}

struct UIApplicationHostObject {
    delegate: id,
}
impl HostObject for UIApplicationHostObject {}

type UIInterfaceOrientation = UIDeviceOrientation;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIApplication: UIResponder

// This should only be called by UIApplicationMain
+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(UIApplicationHostObject {
        delegate: nil,
    });
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

+ (id)sharedApplication {
    env.framework_state.uikit.ui_application.shared_application.unwrap()
}

// This should only be called by UIApplicationMain
- (id)init {
    assert!(env.framework_state.uikit.ui_application.shared_application.is_none());
    env.framework_state.uikit.ui_application.shared_application = Some(this);
    this
}

// This is a singleton, it shouldn't be deallocated.
- (id)retain { this }
- (id)autorelease { this }
- (())release {}

- (id)delegate {
    env.objc.borrow::<UIApplicationHostObject>(this).delegate
}
- (())setDelegate:(id)delegate { // something implementing UIApplicationDelegate
    // This property is quasi-non-retaining: https://stackoverflow.com/a/14271150/736162
    // TODO: release the first delegate, but not any subsequent delegates
    let host_object = env.objc.borrow_mut::<UIApplicationHostObject>(this);
    assert!(host_object.delegate == nil);
    host_object.delegate = delegate;
}

// TODO: statusBarHidden getter
- (())setStatusBarHidden:(bool)_hidden {
    // TODO: store this somewhere
}
- (())setStatusBarHidden:(bool)hidden
                animated:(bool)_animated {
    // TODO: animation
    msg![env; this setStatusBarHidden:hidden]
}

// TODO: statusBarOrientation getter
- (())setStatusBarOrientation:(UIInterfaceOrientation)orientation {
    env.window.rotate_device(match orientation {
        UIDeviceOrientationPortrait => DeviceOrientation::Portrait,
        UIDeviceOrientationLandscapeLeft => DeviceOrientation::LandscapeLeft,
        UIDeviceOrientationLandscapeRight => DeviceOrientation::LandscapeRight,
        _ => unimplemented!("Orientation {} not handled yet", orientation),
    });
}
- (())setStatusBarOrientation:(UIInterfaceOrientation)orientation
                     animated:(bool)_animated {
    // TODO: animation
    msg![env; this setStatusBarOrientation:orientation]
}

- (bool)idleTimerDisabled {
    !env.window.is_screen_saver_enabled()
}
- (())setIdleTimerDisabled:(bool)disabled {
    env.window.set_screen_saver_enabled(!disabled);
}

- (bool)openURL:(id)url { // NSURL
    let ns_string = msg![env; url absoluteURL];
    let url_string = ns_string::to_rust_string(env, ns_string);
    crate::window::open_url(&url_string);

    // iPhone OS doesn't really do multitasking, so the app expects to close
    // when a URL is opened, e.g. Super Monkey Ball keeps opening the URL every
    // frame! Super Monkey Ball also doesn't check whether opening failed, so
    // it's probably best to always exit.
    println!("App opened URL {:?}, exiting.", url_string);
    exit(env);
    true
}

@end

};

/// `UIApplicationMain`, the entry point of the application.
///
/// This function should never return.
pub(super) fn UIApplicationMain(
    env: &mut Environment,
    _argc: i32,
    _argv: MutPtr<MutPtr<u8>>,
    principal_class_name: id, // NSString*
    delegate_class_name: id,  // NSString*
) {
    if principal_class_name != nil || delegate_class_name != nil {
        unimplemented!()
    }

    // UIKit creates and drains autorelease pools when handling events.
    // It's not clear what granularity this should happen with, but this
    // granularity has already caught several bugs. :)

    let (ui_application, delegate) = {
        let pool: id = msg_class![env; NSAutoreleasePool new];

        let ui_application: id = msg_class![env; UIApplication new];

        load_main_nib_file(env, ui_application);

        // The delegate must have been created by this point.
        // While notionally UIApplication does not retain its delegate (see
        // `setDelegate:` above), we do have to retain this first one.
        let delegate: id = msg![env; ui_application delegate];
        assert!(delegate != nil); // should have been set by now
        retain(env, delegate);

        let _: () = msg![env; pool drain];

        (ui_application, delegate)
    };

    {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        () = msg![env; delegate applicationDidFinishLaunching:ui_application];
        let _: () = msg![env; pool drain];
    }

    // Call layoutSubviews on all views in the view hierarchy.
    // See https://medium.com/geekculture/uiview-lifecycle-part-5-faa2d44511c9
    let views = env.framework_state.uikit.ui_view.views.clone();
    for view in views {
        () = msg![env; view layoutSubviews];
    }

    // Send applicationDidBecomeActive now that the application is ready to become active.
    {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        () = msg![env; delegate applicationDidBecomeActive:ui_application];
        let _: () = msg![env; pool drain];
    }

    // FIXME: There are more messages we should send.
    // TODO: Send UIApplicationDidFinishLaunchingNotification?

    // TODO: It might be nicer to return from this function (even though it's
    // conceptually noreturn) and set some global flag that changes how the
    // execution works from this point onwards, though the only real advantages
    // would be a prettier backtrace and maybe the quit button not having to
    // panic.
    let run_loop: id = msg_class![env; NSRunLoop mainRunLoop];
    let _: () = msg![env; run_loop run];
}

/// Tell the app it's about to quit and then exit.
pub(super) fn exit(env: &mut Environment) {
    let ui_application: id = msg_class![env; UIApplication sharedApplication];
    let delegate: id = msg![env; ui_application delegate];

    // FIXME: There are more messages we should send.

    {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        () = msg![env; delegate applicationWillTerminate:ui_application];
        let _: () = msg![env; pool drain];
    }

    std::process::exit(0);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(UIApplicationMain(_, _, _, _))];
