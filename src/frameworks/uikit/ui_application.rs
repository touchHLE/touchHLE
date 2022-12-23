//! `UIApplication` and `UIApplicationMain`.

use crate::mem::MutPtr;
use crate::objc::{id, nil, ClassExports};
use crate::Environment;

pub const CLASSES: ClassExports = crate::objc_classes! {

(env, this, _cmd);

@implementation UIApplication: UIResponder
// TODO
@end

};

/// `UIApplicationMain`, the entry point of the application.
///
/// This function should never return.
pub(super) fn UIApplicationMain(
    _env: &mut Environment,
    _argc: i32,
    _argv: MutPtr<MutPtr<u8>>,
    principal_class_name: id, // NSString*
    delegate_class_name: id,  // NSString*
) {
    if principal_class_name != nil || delegate_class_name != nil {
        unimplemented!()
    }

    unimplemented!("Should enter main loop here")
}
