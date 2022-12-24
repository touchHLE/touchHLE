//! `UINib` and loading of nib files.
//!
//! Resources:
//! - Apple's [Resource Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/LoadingResources/CocoaNibs/CocoaNibs.html) is very helpful.

use crate::frameworks::foundation::ns_keyed_unarchiver;
use crate::objc::id;
use crate::Environment;

// TODO actual UINib class. It's not needed for the main nib file which is
// loaded implicitly.

/// Shortcut for use by [super::ui_application::UIApplicationMain].
///
/// In terms of the proper API, it should behave something like:
/// ```objc
/// UINib *nib = [UINib nibWithName:main_nib_file bundle:nil];
/// return [nib instantiateWithOwner:[UIApplication sharedApplication]
///                     optionsOrNil:nil];
/// ```
///
/// The result value of this function is the `NSArray` of top-level objects.
pub fn load_main_nib_file(env: &mut Environment, _ui_application: id) -> id {
    let path = env.bundle.main_nib_file_path();
    let _dict = ns_keyed_unarchiver::unarchive_object_with_file(env, &path);

    unimplemented!("Finish nib loading");
}
