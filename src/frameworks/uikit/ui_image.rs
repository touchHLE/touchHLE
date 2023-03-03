/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIImage`.

use crate::frameworks::foundation::ns_string;
use crate::fs::GuestPath;
use crate::image::Image;
use crate::mem::MutVoidPtr;
use crate::objc::{autorelease, id, msg, msg_class, nil, objc_classes, ClassExports, HostObject};

struct UIImageHostObject {
    image: Option<Image>,
}
impl HostObject for UIImageHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImage: NSObject

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(UIImageHostObject { image: None });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)imageNamed:(id)name { // NSString*
    // TODO: figure out whether this is actually correct in all cases
    let bundle: id = msg_class![env; NSBundle mainBundle];
    let path: id = msg![env; bundle pathForResource:name ofType:nil];
    msg![env; this imageWithContentsOfFile:path]
}

+ (id)imageWithContentsOfFile:(id)path { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfFile:path];
    autorelease(env, new)
}

- (id)initWithContentsOfFile:(id)path { // NSString*
    let path = ns_string::to_rust_string(env, path); // TODO: avoid copy
    // TODO: Real error handling. For now, most errors are likely to be caused
    //       by a functionality gap in touchHLE, not the app actually trying to
    //       load a missing or broken file, so panicking is most useful.
    let bytes = env.fs.read(GuestPath::new(&path)).unwrap();
    let image = Image::from_bytes(&bytes).unwrap();
    env.objc.borrow_mut::<UIImageHostObject>(this).image = Some(image);
    this
}

// TODO: more init methods
// TODO: accessors

@end

};
