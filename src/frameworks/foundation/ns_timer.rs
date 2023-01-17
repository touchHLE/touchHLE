//! `NSTimer`.

use super::ns_run_loop::NSDefaultRunLoopMode;
use super::ns_string;
use super::NSTimeInterval;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    SEL,
};
use crate::Environment;

struct NSTimerHostObject {
    interval: NSTimeInterval,
    /// Strong reference
    target: id,
    _selector: SEL,
    /// Strong reference
    user_info: id,
    repeats: bool,
    valid: bool,
    /// Weak reference
    run_loop: id,
}
impl HostObject for NSTimerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSTimer doesn't seem to be an abstract class?
@implementation NSTimer: NSObject

+ (id)timerWithTimeInterval:(NSTimeInterval)interval
                     target:(id)target
                   selector:(SEL)selector
                   userInfo:(id)user_info
                    repeats:(bool)repeats {
    let interval = interval.max(0.0001);

    retain(env, target);
    retain(env, user_info);

    let host_object = Box::new(NSTimerHostObject {
        interval,
        target,
        _selector: selector,
        user_info,
        repeats,
        valid: true,
        run_loop: nil,
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!(
        "New {} timer {:?}, interval {}s, target [{:?} {}], user info {:?}",
        if repeats { "repeating" } else { "single-use" },
        new,
        interval,
        target,
        selector.as_str(&env.mem),
        user_info,
    );

    autorelease(env, new)
}

+ (id)scheduledTimerWithTimeInterval:(NSTimeInterval)interval
                              target:(id)target
                            selector:(SEL)selector
                            userInfo:(id)user_info
                             repeats:(bool)repeats {
    let timer = msg![env; this timerWithTimeInterval:interval
                                              target:target
                                            selector:selector
                                            userInfo:user_info
                                             repeats:repeats];

    let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
    let mode: id = ns_string::get_static_str(env, NSDefaultRunLoopMode);
    let _: () = msg![env; run_loop addTimer:timer forMode:mode];

    timer
}

- (())dealloc {
    let &NSTimerHostObject {
        target,
        user_info,
        run_loop,
        ..
    } = env.objc.borrow(this);
    release(env, target);
    release(env, user_info);
    assert!(run_loop == nil); // run loop should remove itself (TODO)
    env.objc.dealloc_object(this, &mut env.mem)
}

- (NSTimeInterval)timeInterval {
    let host_object = env.objc.borrow::<NSTimerHostObject>(this);
    if host_object.repeats {
        host_object.interval
    } else {
        0.0 // this is the documented behaviour!
    }
}
- (id)userInfo {
    env.objc.borrow::<NSTimerHostObject>(this).user_info
}
- (bool)isValid {
    env.objc.borrow::<NSTimerHostObject>(this).valid
}

// TODO: more constructors
// TODO: more accessors
// TODO: actually run the timer

@end

};

/// For use by `NSRunLoop`
pub(super) fn set_run_loop(env: &mut Environment, timer: id, run_loop: id) {
    let host_object = env.objc.borrow_mut::<NSTimerHostObject>(timer);
    assert!(host_object.run_loop == nil); // TODO: what do we do here?
    host_object.run_loop = run_loop;
}
