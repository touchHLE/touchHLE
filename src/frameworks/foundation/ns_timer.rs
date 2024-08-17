/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSTimer`.

use super::ns_run_loop::NSDefaultRunLoopMode;
use super::NSTimeInterval;
use super::{ns_run_loop, ns_string};
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, SEL,
};
use crate::Environment;
use std::time::{Duration, Instant};

struct NSTimerHostObject {
    ns_interval: NSTimeInterval,
    /// Copy of `ns_interval` in Rust's type for time intervals. Keep in sync!
    rust_interval: Duration,
    /// Strong reference
    target: id,
    selector: SEL,
    /// Strong reference
    user_info: id,
    repeats: bool,
    due_by: Option<Instant>,
    /// Weak reference
    run_loop: id,
}
impl HostObject for NSTimerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSTimer doesn't seem to be an abstract class?
@implementation NSTimer: NSObject

+ (id)timerWithTimeInterval:(NSTimeInterval)ns_interval
                     target:(id)target
                   selector:(SEL)selector
                   userInfo:(id)user_info
                    repeats:(bool)repeats {
    let ns_interval = ns_interval.max(0.0001);
    let rust_interval = Duration::from_secs_f64(ns_interval);

    retain(env, target);
    retain(env, user_info);

    let host_object = Box::new(NSTimerHostObject {
        ns_interval,
        rust_interval,
        target,
        selector,
        user_info,
        repeats,
        due_by: Some(Instant::now().checked_add(rust_interval).unwrap()),
        run_loop: nil,
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!(
        "New {} timer {:?}, interval {}s, target [{:?} {}], user info {:?}",
        if repeats { "repeating" } else { "single-use" },
        new,
        ns_interval,
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
        ..
    } = env.objc.borrow(this);
    release(env, target);
    release(env, user_info);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (NSTimeInterval)timeInterval {
    let host_object = env.objc.borrow::<NSTimerHostObject>(this);
    if host_object.repeats {
        host_object.ns_interval
    } else {
        0.0 // this is the documented behaviour!
    }
}
- (id)userInfo {
    env.objc.borrow::<NSTimerHostObject>(this).user_info
}
- (bool)isValid {
    env.objc.borrow::<NSTimerHostObject>(this).due_by.is_some()
}

- (())invalidate {
    // Timer might already be invalid, don't try to remove it twice.
    if env.objc.borrow_mut::<NSTimerHostObject>(this).due_by.take().is_some() {
        let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
        ns_run_loop::remove_timer(env, run_loop, this);
    }
}

- (())fire {
    let &NSTimerHostObject {
        target,
        selector,
        repeats,
        ..
    } = env.objc.borrow(this);

    let pool: id = msg_class![env; NSAutoreleasePool new];

    // Signature should be `- (void)timerDidFire:(NSTimer *)which`.
    let _: () = msg_send(env, (target, selector, this));

    release(env, pool);

    if !repeats {
        () = msg![env; this invalidate];
    }
}

// TODO: more constructors
// TODO: more accessors

@end

};

/// For use by `NSRunLoop`
pub(super) fn set_run_loop(env: &mut Environment, timer: id, run_loop: id) {
    let host_object = env.objc.borrow_mut::<NSTimerHostObject>(timer);
    assert!(host_object.run_loop == nil); // TODO: what do we do here?
    host_object.run_loop = run_loop;
}

/// For use by `NSRunLoop`: check if a timer is due to fire and fire it if
/// necessary.
///
/// Returns the next firing time, if any.
pub(super) fn handle_timer(env: &mut Environment, timer: id) -> Option<Instant> {
    let &NSTimerHostObject {
        ns_interval,
        rust_interval,
        target,
        selector,
        repeats,
        due_by,
        run_loop,
        ..
    } = env.objc.borrow(timer);

    // invalidated timers should have already been removed from the run loop
    let due_by = due_by.unwrap();

    let now = Instant::now();

    if due_by > now {
        return Some(due_by);
    }

    let overdue_by = now.duration_since(due_by);

    // Timer may be released when it's invalidated, so we need to retain it so
    // it's still around to pass to the timer target.
    retain(env, timer);

    // Advancing the timer before sending its message seems like a good idea
    // considering this function is potentially re-entrant.
    let new_due_by = if repeats {
        // When rescheduling a repeating timer, the next firing should be based
        // on when the timer should have fired, not when it actually fired, so
        // that there is no drift over time.
        //
        // For example, if a timer has an interval of 60s and starts at 00:00,
        // the first firing would be scheduled for 01:00, and the second firing
        // should be scheduled for 02:00, even if the first firing was at 01:01.
        //
        // However: if the timer handling is delayed past a whole interval, it
        // should not try to catch up. For example, if the first firing is
        // scheduled for 01:00 but happens at 02:30, then the next firing should
        // be scheduled for 03:00.
        // TODO: Use `.div_duration_f64()` once that is stabilized.
        let advance_by = (overdue_by.as_secs_f64() / ns_interval).max(1.0).ceil();
        assert!(advance_by == (advance_by as u32) as f64);
        let advance_by = advance_by as u32;
        if advance_by > 1 {
            log_dbg!("Warning: Timer {:?} is lagging. It is overdue by {}s and has missed {} interval(s)!", timer, overdue_by.as_secs_f64(), advance_by - 1);
        }
        let advance_by = rust_interval.checked_mul(advance_by).unwrap();
        Some(due_by.checked_add(advance_by).unwrap())
    } else {
        ns_run_loop::remove_timer(env, run_loop, timer);
        None
    };
    env.objc.borrow_mut::<NSTimerHostObject>(timer).due_by = new_due_by;

    log_dbg!(
        "Timer {:?} fired, sending {:?} message to {:?}",
        timer,
        selector.as_str(&env.mem),
        target
    );

    let pool: id = msg_class![env; NSAutoreleasePool new];

    // Signature should be `- (void)timerDidFire:(NSTimer *)which`.
    let _: () = msg_send(env, (target, selector, timer));

    release(env, timer);
    release(env, pool);

    new_due_by
}
