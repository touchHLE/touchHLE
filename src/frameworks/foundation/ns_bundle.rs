//! `NSBundle`.

use super::ns_string::get_static_str;
use crate::bundle::Bundle;
use crate::objc::{id, msg, msg_class, objc_classes, release, ClassExports, HostObject};

#[derive(Default)]
pub struct State {
    main_bundle: Option<id>,
}

struct NSBundleHostObject {
    /// If this is [None], this is the main bundle's NSBundle instance and the
    /// [Bundle] is stored in [crate::Environment], not here.
    _bundle: Option<Bundle>,
    /// NSString with bundle path.
    bundle_path: id,
    /// NSURL with bundle path. [None] if not created yet.
    bundle_url: Option<id>,
}
impl HostObject for NSBundleHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSBundle: NSObject

+ (id)mainBundle {
    if let Some(bundle) = env.framework_state.foundation.ns_bundle.main_bundle {
        bundle
    } else {
        let bundle_path = get_static_str(env, crate::APP_PATH);
        let host_object = NSBundleHostObject {
            _bundle: None,
            bundle_path,
            bundle_url: None,
        };
        let new = env.objc.alloc_object(
            this,
            Box::new(host_object),
            &mut env.mem
        );
        env.framework_state.foundation.ns_bundle.main_bundle = Some(new);
        new
   }
}

- (())dealloc {
    let &NSBundleHostObject { bundle_url, .. } = env.objc.borrow(this);
    if let Some(bundle_url) = bundle_url {
        release(env, bundle_url);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)bundlePath {
    env.objc.borrow::<NSBundleHostObject>(this).bundle_path
}
- (id)bundleURL {
    if let Some(url) = env.objc.borrow::<NSBundleHostObject>(this).bundle_url {
        url
    } else {
        let bundle_path: id = msg![env; this bundlePath];
        let new: id = msg_class![env; NSURL alloc];
        let new: id = msg![env; new initFileURLWithPath:bundle_path];
        env.objc.borrow_mut::<NSBundleHostObject>(this).bundle_url = Some(new);
        new
    }
}

- (id)resourcePath {
    // This seems to be the same as the bundle path. The iPhone OS bundle
    // structure is a lot flatter than the macOS one.
    msg![env; this bundlePath]
}
- (id)resourceURL {
    // This seems to be the same as the bundle path. The iPhone OS bundle
    // structure is a lot flatter than the macOS one.
    msg![env; this bundleURL]
}

// TODO: constructors, more accessors

@end

};
