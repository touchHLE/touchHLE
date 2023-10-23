/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::NSComparisonResult;
use crate::objc::{id, msg, msg_send, nil, ClassExports, HostObject, NSZonePtr, SEL};
use crate::objc_classes;

struct SortDescriptorObject {
    key: id,
    ascending: bool,
    selector: SEL,
}
impl HostObject for SortDescriptorObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSSortDescriptor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SortDescriptorObject {
        key: nil,
        ascending: false,
        selector: SEL::null(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithKey:(id)key
        ascending:(bool)asc {
    let sel = env.objc.register_host_selector("compare:".to_string(), &mut env.mem);
    msg![env; this initWithKey: key ascending: asc selector: sel]
}

- (id)initWithKey:(id)key
        ascending:(bool)asc
         selector:(SEL)sel {
    let host_obj = env.objc.borrow_mut::<SortDescriptorObject>(this);
    host_obj.key = key;
    host_obj.selector = sel;
    host_obj.ascending = asc;
    this
}

-(NSComparisonResult)compareObject:(id)left
                          toObject:(id)right {
    let &SortDescriptorObject{
        key, ascending, selector
    } = env.objc.borrow(this);
    let left_key: id = msg![env; left valueForKey: key];
    let right_key: id = msg![env; right valueForKey: key];
    let result: NSComparisonResult = msg_send(env, (left_key, selector, right_key));
    if !ascending {
        -result
    } else {
        result
    }
}
@end
};
