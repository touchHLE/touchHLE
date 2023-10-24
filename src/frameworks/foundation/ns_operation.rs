/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, objc_classes, ClassExports, SEL, HostObject, retain, release, nil, NSZonePtr, msg, msg_class,
};


struct InvocationOperationHost {
    target: id,
    selector: SEL,
    argument: id,
}
impl HostObject for InvocationOperationHost {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSOperation: NSObject

-(())start {
    msg![env; this main]
}

- (())main {
    // Apple Docs: In your implementation, do not invoke super.
}

@end

@implementation NSInvocationOperation: NSOperation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(InvocationOperationHost {
        target: nil,
        argument: nil,
        selector: SEL::null(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTarget:(id)target
            selector:(SEL)selector
            object:(id)argument {
    retain(env, target);
    retain(env, argument);
    let host = env.objc.borrow_mut::<InvocationOperationHost>(this);
    host.argument = argument;
    host.target = target;
    host.selector = selector;
    this
}

- (())main {
    let &InvocationOperationHost {argument, target, selector} = env.objc.borrow(this);
    if argument == nil {
        let _: id = msg![env; target performSelector: selector];
    } else {
        let _: id = msg![env; target performSelector: selector withObject: argument];
    }
}

- (())dealloc {
    let &InvocationOperationHost{argument, target, ..} = env.objc.borrow(this);
    release(env, argument);
    release(env, target);

    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation NSOperationQueue: NSObject

- (())addOperation:(id)op {
    let opclass = msg![env; op class];
    dbg!(env.objc.get_class_name(opclass));
    let thread = msg_class![env; NSThread alloc];
    let sel = env.objc.register_host_selector("_startOperation:".to_string(), &mut env.mem);
    let thread = msg![env; thread initWithTarget: this selector: sel object: op];
    () = msg![env; thread start];
    release(env, thread);
}

- (())_startOperation:(id)op {
    msg![env; op start]
}

-(())setMaxConcurrentOperationCount: (NSInteger)_count {

}
@end

};
