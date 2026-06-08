/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSSortDescriptor`.

use super::{NSComparisonResult, NSOrderedAscending, NSOrderedDescending};
use crate::objc::{
    autorelease, id, msg, msg_send, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};

struct NSSortDescriptorHostObject {
    /// NSString*, or nil to compare objects directly.
    key: id,
    ascending: bool,
    selector: SEL,
}
impl HostObject for NSSortDescriptorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSSortDescriptor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let selector = env.objc.register_host_selector("compare:".into(), &mut env.mem);
    let host_object = NSSortDescriptorHostObject {
        key: nil,
        ascending: true,
        selector,
    };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

+ (id)sortDescriptorWithKey:(id)key
                  ascending:(bool)ascending {
    let descriptor: id = msg![env; this alloc];
    let descriptor: id = msg![env; descriptor initWithKey:key ascending:ascending];
    autorelease(env, descriptor)
}

+ (id)sortDescriptorWithKey:(id)key
                  ascending:(bool)ascending
                   selector:(SEL)selector {
    let descriptor: id = msg![env; this alloc];
    let descriptor: id = msg![env; descriptor initWithKey:key ascending:ascending selector:selector];
    autorelease(env, descriptor)
}

- (id)initWithKey:(id)key
        ascending:(bool)ascending {
    let selector = env.objc.register_host_selector("compare:".into(), &mut env.mem);
    msg![env; this initWithKey:key ascending:ascending selector:selector]
}

- (id)initWithKey:(id)key
        ascending:(bool)ascending
         selector:(SEL)selector {
    if key != nil {
        retain(env, key);
    }
    let old_key = {
        let host_object = env.objc.borrow_mut::<NSSortDescriptorHostObject>(this);
        let old_key = host_object.key;
        host_object.key = key;
        host_object.ascending = ascending;
        host_object.selector = selector;
        old_key
    };
    release(env, old_key);
    this
}

- (())dealloc {
    let key = env.objc.borrow::<NSSortDescriptorHostObject>(this).key;
    release(env, key);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)key {
    env.objc.borrow::<NSSortDescriptorHostObject>(this).key
}

- (bool)ascending {
    env.objc.borrow::<NSSortDescriptorHostObject>(this).ascending
}

- (SEL)selector {
    env.objc.borrow::<NSSortDescriptorHostObject>(this).selector
}

- (NSComparisonResult)compareObject:(id)object1
                           toObject:(id)object2 {
    let &NSSortDescriptorHostObject {
        key,
        ascending,
        selector,
    } = env.objc.borrow(this);

    let (value1, value2) = if key == nil {
        (object1, object2)
    } else {
        let value1: id = msg![env; object1 valueForKey:key];
        let value2: id = msg![env; object2 valueForKey:key];
        (value1, value2)
    };

    let mut result: NSComparisonResult = msg_send(env, (value1, selector, value2));
    if !ascending {
        result = match result {
            NSOrderedAscending => NSOrderedDescending,
            NSOrderedDescending => NSOrderedAscending,
            other => other,
        };
    }
    result
}

@end

};
