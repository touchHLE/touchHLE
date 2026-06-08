/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPasteboard`.

use crate::objc::{id, msg, nil, objc_classes, retain, ClassExports, HostObject, NSZonePtr};

struct UIPasteboardHostObject {
    name: id,
    string_val: id,
}
impl HostObject for UIPasteboardHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPasteboard: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPasteboardHostObject { name: nil, string_val: nil });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)pasteboardWithName:(id)name create:(bool)_create {
    log_dbg!("TODO: [UIPasteboard pasteboardWithName:{:?} create:{}]", name, _create);
    let instance: id = msg![env; this alloc];
    let instance = crate::objc::autorelease(env, instance);
    env.objc.borrow_mut::<UIPasteboardHostObject>(instance).name = retain(env, name);
    instance
}

+ (id)generalPasteboard {
    let name = crate::frameworks::foundation::ns_string::get_static_str(env, "general");
    msg![env; this pasteboardWithName:name create:true]
}

- (id)string {
    env.objc.borrow::<UIPasteboardHostObject>(this).string_val
}

- (())setString:(id)string {
    let string = retain(env, string);
    let old_string = std::mem::replace(
        &mut env.objc.borrow_mut::<UIPasteboardHostObject>(this).string_val,
        string,
    );
    crate::objc::release(env, old_string);
}

@end

};
