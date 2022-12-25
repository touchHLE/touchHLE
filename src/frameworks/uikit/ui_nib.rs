//! `UINib` and loading of nib files.
//!
//! Resources:
//! - Apple's [Resource Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/LoadingResources/CocoaNibs/CocoaNibs.html) is very helpful.
//! - GitHub user 0xced's [reverse-engineering of UIClassSwapper](https://gist.github.com/0xced/45daf79b62ad6a20be1c).

use crate::frameworks::foundation::ns_keyed_unarchiver;
use crate::objc::{id, objc_classes, ClassExports};
use crate::Environment;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// TODO actual UINib class. It's not needed for the main nib file which is
// loaded implicitly.

// An undocumented type that nib files reference by name. NSKeyedUnarchiver will
// find and instantiate this class.
@implementation UIProxyObject: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIProxyObject*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

// Another undocumented type used by nib files. This one seems to be used to
// instantiate types that don't implement NSCoding (i.e. don't respond to
// initWithCoder:). See the link at the top of this file.
@implementation UIClassSwapper: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIClassSwapper*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

// Another undocumented type used by nib files.
@implementation UIRuntimeOutletConnection: NSObject

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    eprintln!("TODO: [(UIRuntimeOutletConnection*){:?} initWithCoder:{:?}]", this, coder);
    this
}

@end

};

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
    let dict = ns_keyed_unarchiver::unarchive_object_with_file(env, &path);

    unimplemented!("Finish nib loading with {:#?}", dict);
}
