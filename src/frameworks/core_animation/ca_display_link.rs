/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CADisplayLink`

use crate::frameworks::foundation::ns_run_loop::NSRunLoopMode;
use crate::frameworks::foundation::ns_timer::set_time_interval;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr, SEL,
};

#[derive(Default)]
struct CADisplayLinkHostObject {
    target: id,
    selector: Option<SEL>,
    /// Weak reference. The timer retains the display link (as its target),
    /// so the timer necessarily outlives the display link. After `invalidate`,
    /// this pointer must not be used.
    ns_timer: id,
    paused: bool,
}
impl HostObject for CADisplayLinkHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CADisplayLink: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::new(CADisplayLinkHostObject::default()), &mut env.mem)
}

+ (id)displayLinkWithTarget:(id)target selector:(SEL)sel {
    let display_link: id = msg![env; this new];
    // Because timer will pass itself as a second arg in ns_timer:handle_timer,
    // we need to use a re-direction: first fire the timer on the display link,
    // then call the original selector, passing the link as a second argument!
    let redirect_sel: SEL = env.objc.lookup_selector("_touchHLE_displayLinkTimerDidFire:").unwrap();
    let ns_timer = msg_class![env; NSTimer timerWithTimeInterval:(1.0/60.0)
                     target:display_link
                   selector:redirect_sel
                   userInfo:nil
                    repeats:true];
    retain(env, target);
    let host_object = env.objc.borrow_mut::<CADisplayLinkHostObject>(display_link);
    host_object.target = target;
    host_object.selector = Some(sel);
    host_object.ns_timer = ns_timer;
    log_dbg!("[CADisplayLink displayLinkWithTarget:{:?} selector:{}] => {:?}", target, sel.as_str(&env.mem), display_link);
    autorelease(env, display_link)
}

- (bool)isPaused {
    env.objc.borrow::<CADisplayLinkHostObject>(this).paused
}
- (())setPaused:(bool)paused {
    env.objc.borrow_mut::<CADisplayLinkHostObject>(this).paused = paused;
}

- (())setFrameInterval:(NSInteger)frameInterval {
    log_dbg!("[(CADisplayLink*){:?} setFrameInterval:{}]", this, frameInterval);
    assert!(frameInterval >= 1);
    let interval = frameInterval as f64 / 60.0;
    let ns_timer = env.objc.borrow::<CADisplayLinkHostObject>(this).ns_timer;
    set_time_interval(env, ns_timer, interval);
}

- (())addToRunLoop:(id)run_loop forMode:(NSRunLoopMode)mode {
    log_dbg!("[(CADisplayLink*){:?} addToRunLoop:{:?} forMode:{:?}]", this, run_loop, mode);
    let ns_timer = env.objc.borrow::<CADisplayLinkHostObject>(this).ns_timer;
    () = msg![env; run_loop addTimer:ns_timer forMode:mode];
}

- (())invalidate {
    log_dbg!("[(CADisplayLink*){:?} invalidate]", this);
    let ns_timer = env.objc.borrow::<CADisplayLinkHostObject>(this).ns_timer;
    () = msg![env; ns_timer invalidate];
}

- (())dealloc {
    let &CADisplayLinkHostObject { target, .. } = env.objc.borrow(this);
    release(env, target);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (())_touchHLE_displayLinkTimerDidFire:(id)timer { // NSTimer *
    let &CADisplayLinkHostObject {
        target,
        selector,
        ns_timer,
        paused,
        ..
    } = env.objc.borrow::<CADisplayLinkHostObject>(this);
    assert_eq!(ns_timer, timer);
    if paused {
        // This could be improved, as we're still running the timer,
        // but just not passing the actual call.
        return;
    }
    // Signature is `- (void) selector:(CADisplayLink *)sender;`
    () = msg_send(env, (target, selector.unwrap(), this));
}

@end

};
