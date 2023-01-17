//! `UIApplication` and `UIApplicationMain`.

use super::ui_device::*;
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

    // TODO: Are there more messages we need to send?
    // TODO: Send UIApplicationDidFinishLaunchingNotification?

    unimplemented!("Send more messages and enter main loop");
}
