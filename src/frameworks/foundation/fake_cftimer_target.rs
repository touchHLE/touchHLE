use crate::abi::{CallFromHost, GuestFunction};
use crate::mem::MutVoidPtr;
use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

struct FakeCFTimerTargetHostObject {
    callout: GuestFunction,
    context: MutVoidPtr,
}
impl HostObject for FakeCFTimerTargetHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation FakeCFTimerTarget: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(FakeCFTimerTargetHostObject {
        callout: GuestFunction::from_addr_with_thumb_bit(0),
        context: MutVoidPtr::null()
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCallout:(GuestFunction)callout context:(MutVoidPtr)context {
    let host_object: &mut FakeCFTimerTargetHostObject = env.objc.borrow_mut(this);
    host_object.callout = callout;
    host_object.context = context;
    this
}

- (())timerFireMethod:(id)timer { // NSTimer *
    let &FakeCFTimerTargetHostObject {
        callout,
        context
    } = env.objc.borrow(this);
    () = callout.call_from_host(env, (timer, context));
}

@end

};
