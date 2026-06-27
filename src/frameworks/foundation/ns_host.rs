/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSHost`.
//!
//! A minimal implementation: `+[NSHost currentHost]` returns a singleton whose
//! `-name` is `"localhost"` and whose `-address` is `"127.0.0.1"`.  This is
//! sufficient for apps that query the host name for analytics or display.

use crate::frameworks::foundation::ns_string;
use crate::objc::{autorelease, id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
struct NSHostHostObject {
    name: id,
    address: id,
}
impl HostObject for NSHostHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSHost: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSHostHostObject {
        name: ns_string::get_static_str(env, "localhost"),
        address: ns_string::get_static_str(env, "127.0.0.1"),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)currentHost {
    let host: id = msg_class![env; NSHost alloc];
    let host: id = msg![env; host init];
    autorelease(env, host)
}

- (id)name {
    env.objc.borrow::<NSHostHostObject>(this).name
}

- (id)address {
    env.objc.borrow::<NSHostHostObject>(this).address
}

- (id)addresses {
    let addr = env.objc.borrow::<NSHostHostObject>(this).address;
    let arr: id = msg_class![env; NSArray arrayWithObject:addr];
    arr
}

- (id)names {
    let name = env.objc.borrow::<NSHostHostObject>(this).name;
    let arr: id = msg_class![env; NSArray arrayWithObject:name];
    arr
}

@end

};
