//! `UIApplication` and `UIApplicationMain`.

use crate::frameworks::uikit::ui_nib::load_main_nib_file;
use crate::mem::MutPtr;
use crate::objc::{id, msg_class, nil, objc_classes, ClassExports};
use crate::Environment;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIApplication: UIResponder

// This should only be called by UIApplicationMain
- (id)init {
    // TODO: handle the fact this is a singleton
    this
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

    let ui_application: id = msg_class![env; UIApplication new];

    load_main_nib_file(env, ui_application);

    unimplemented!("Send events to UIApplicationDelegate and enter main loop");
}
