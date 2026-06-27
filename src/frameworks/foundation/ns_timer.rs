/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSTimer`.

use super::NSTimeInterval;
use super::ns_time_interval_to_duration_or_zero;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr, SEL,
};
use crate::Environment;
use std::time::{Duration, Instant};

#[derive(Default)]
struct NSTimerHostObject {
    ns_interval: NSTimeInterval,
    /// Copy of `ns_interval` in Rust's type for time intervals. Keep in sync!
    rust_interval: Duration,
    /// Strong reference
    target: id,
    selector: SEL,
    /// Strong reference
    user_info: id,
    /// Strong reference to an `NSInvocation*`, used by the
    /// `…:invocation:repeats:` variants. When non-nil, firing the timer calls
    /// `[invocation invoke]` instead of sending `selector` to `target`.
    invocation: id,
    repeats: bool,
    due_by: Option<Instant>,
    /// If the timer is currently running its callback, this is set so that the
    /// re-entering the run loop from inside the callback doesn't cause an
    /// infinite loop.
    is_running_callback: bool,
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
    // Sanitize the interval before handing it to `Duration::from_secs_f64`,
    // which panics on NaN, infinity, negative, or huge values. iPhone OS
    // historically clamps such values silently (Crazy Frog Racer
    // — HyperHLE log #4 — sends a non-finite value here on first
    // NSTimer fire), so do the same instead of aborting the emulator.
    let ns_interval = if ns_interval.is_finite() && ns_interval > 0.0001 {
        ns_interval
    } else {
        0.0001
    };
    let rust_interval = ns_time_interval_to_duration_or_zero(ns_interval);

    retain(env, target);
    retain(env, user_info);

    let due_by = Instant::now()
        .checked_add(rust_interval)
        .unwrap_or_else(Instant::now);
    let host_object = Box::new(NSTimerHostObject {
        ns_interval,
        rust_interval,
        target,
        selector,
        user_info,
        invocation: nil,
        repeats,
        due_by: Some(due_by),
        run_loop: nil,
        is_running_callback: false
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

+ (id)allocWithZone:(NSZonePtr)_zone {
    // Безопасно зануляем структуру
    let host_object = Box::new(unsafe { std::mem::zeroed::<NSTimerHostObject>() });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)scheduledTimerWithTimeInterval:(f64)ti target:(id)t selector:(SEL)s userInfo:(id)ui repeats:(bool)rep {
    let timer: id = msg_class![env; NSTimer alloc];
    let timer: id = msg![env; timer initWithFireDate:nil interval:ti target:t selector:s userInfo:ui repeats:rep];

    let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];

    // Получаем реальный гостевой NSString из системного пула эмулятора
    let mode_str = crate::frameworks::foundation::ns_string::get_static_str(env, "NSDefaultRunLoopMode");
    let _: () = msg![env; run_loop addTimer:timer forMode:mode_str];

    autorelease(env, timer)
}

+ (id)timerWithTimeInterval:(NSTimeInterval)ns_interval
                 invocation:(id)invocation
                    repeats:(bool)repeats {
    // Like `timerWithTimeInterval:target:selector:userInfo:repeats:`, but the
    // timer fires `[invocation invoke]` instead of a target/selector pair.
    // Speak & Type (com.vinerbi.iphoneparlante) uses this to drive its splash
    // screen, and stayed stuck on the splash when this returned nil.
    let ns_interval = if ns_interval.is_finite() && ns_interval > 0.0001 {
        ns_interval
    } else {
        0.0001
    };
    let rust_interval = ns_time_interval_to_duration_or_zero(ns_interval);

    retain(env, invocation);

    let due_by = Instant::now()
        .checked_add(rust_interval)
        .unwrap_or_else(Instant::now);
    let host_object = Box::new(NSTimerHostObject {
        ns_interval,
        rust_interval,
        target: nil,
        selector: SEL::null(),
        user_info: nil,
        invocation,
        repeats,
        due_by: Some(due_by),
        run_loop: nil,
        is_running_callback: false,
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!(
        "New {} invocation timer {:?}, interval {}s, invocation {:?}",
        if repeats { "repeating" } else { "single-use" },
        new,
        ns_interval,
        invocation,
    );
    autorelease(env, new)
}

+ (id)scheduledTimerWithTimeInterval:(NSTimeInterval)ti
                          invocation:(id)invocation
                             repeats:(bool)rep {
    let timer: id = msg_class![env; NSTimer timerWithTimeInterval:ti invocation:invocation repeats:rep];

    let run_loop: id = msg_class![env; NSRunLoop currentRunLoop];
    let mode_str = crate::frameworks::foundation::ns_string::get_static_str(env, "NSDefaultRunLoopMode");
    let _: () = msg![env; run_loop addTimer:timer forMode:mode_str];

    timer
}

- (())dealloc {
    let _: () = msg![env; this invalidate];

    // ИСПРАВЛЕНИЕ: Используем блок для освобождения заимствования до вызова
    // release
    let (target, user_info, invocation) = {
        let host = env.objc.borrow::<NSTimerHostObject>(this);
        (host.target, host.user_info, host.invocation)
    }; // Здесь заимствование уничтожается

    release(env, target);
    release(env, user_info);
    release(env, invocation);
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

- (id)target {
    env.objc.borrow::<NSTimerHostObject>(this).target
}

- (bool)isValid {
    env.objc.borrow::<NSTimerHostObject>(this).due_by.is_some()
}

- (bool)isCancelled {
        let is_valid = env.objc.borrow::<NSTimerHostObject>(this).due_by.is_some();
        !is_valid
}

- (())invalidate {
    let run_loop_to_remove = {
        let host = env.objc.borrow_mut::<NSTimerHostObject>(this);
        host.due_by = None;
        let rl = host.run_loop;
        host.run_loop = crate::objc::nil;
        rl
    };

    if run_loop_to_remove != crate::objc::nil {
        crate::frameworks::foundation::ns_run_loop::remove_timer(env, run_loop_to_remove, this);
    }
}

- (())fire {
    let &NSTimerHostObject {
        target,
        selector,
        invocation,
        repeats,
        ..
    } = env.objc.borrow(this);
    let pool: id = msg_class![env; NSAutoreleasePool new];

    if invocation != nil {
        let _: () = msg![env; invocation invoke];
    } else {
        // Signature should be `- (void)timerDidFire:(NSTimer *)which`.
        let _: () = msg_send(env, (target, selector, this));
    }

    release(env, pool);
    if !repeats {
        () = msg![env; this invalidate];
    }
}

// MARK: - Fire Date

- (())setFireDate:(id)date {
    // ВЫЧИСЛЯЕМ до borrow_mut
    let time_interval: NSTimeInterval = msg![env; date timeIntervalSinceNow];

    let timer = env.objc.borrow_mut::<NSTimerHostObject>(this);
    if timer.due_by.is_some() {
        if !time_interval.is_finite() || time_interval <= 0.0 {
            timer.due_by = Some(Instant::now());
        } else {
            let safe_interval = time_interval.min(100.0 * 365.0 * 24.0 * 3600.0);
            let delta = ns_time_interval_to_duration_or_zero(safe_interval);
            timer.due_by = Instant::now().checked_add(delta).or(Some(Instant::now()));
        }
    }
}

- (id)fireDate {
    // ИСПРАВЛЕНИЕ: Получаем due_by и сразу освобождаем заимствование
    let due_by_opt = {
        env.objc.borrow::<NSTimerHostObject>(this).due_by
    };

    if let Some(due) = due_by_opt {
        let now = Instant::now();
        let time_interval: NSTimeInterval = if due > now {
            due.duration_since(now).as_secs_f64()
        } else {
            -now.duration_since(due).as_secs_f64()
        };
        // Теперь безопасно вызывать макрос с env
        msg_class![env; NSDate dateWithTimeIntervalSinceNow:time_interval]
    } else {
        nil
    }
}

- (id)initWithFireDate:(id)_date interval:(f64)ti target:(id)t selector:(SEL)s userInfo:(id)ui repeats:(bool)rep {
    let this: id = crate::objc::msg_super![env; this init];

    let retained_target = retain(env, t);
    let retained_user_info = retain(env, ui);

    let safe_ti = if ti.is_finite() && ti > 0.0001 { ti } else { 0.0001 };
    let rust_interval = ns_time_interval_to_duration_or_zero(safe_ti);

    // ИСПРАВЛЕНИЕ E0499: Вычисляем fire_time ДО того, как берём `borrow_mut`
    let fire_time = if _date != crate::objc::nil {
        // Здесь безопасно использовать env, так как мы еще ничего не
        // позаимствовали
        let time_interval: NSTimeInterval = msg![env; _date timeIntervalSinceNow];
        if !time_interval.is_finite() || time_interval <= 0.0 {
            std::time::Instant::now()
        } else {
            let safe_interval = time_interval.min(100.0 * 365.0 * 24.0 * 3600.0);
            let delta = ns_time_interval_to_duration_or_zero(safe_interval);
            std::time::Instant::now()
                .checked_add(delta)
                .unwrap_or_else(std::time::Instant::now)
        }
    } else {
        std::time::Instant::now()
            .checked_add(rust_interval)
            .unwrap_or_else(std::time::Instant::now)
    };

    // ТОЛЬКО ТЕПЕРЬ берём `borrow_mut` и записываем все данные
    let host = env.objc.borrow_mut::<NSTimerHostObject>(this);

    host.ns_interval = safe_ti;
    host.rust_interval = rust_interval;
    host.target = retained_target;
    host.selector = s;
    host.user_info = retained_user_info;
    host.repeats = rep;
    host.due_by = Some(fire_time);

    this
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    // ИСПРАВЛЕНИЕ: Формируем строку внутри блока, освобождаем borrow, потом
    // вызываем msg_class!
    let s = {
        let host = env.objc.borrow::<NSTimerHostObject>(this);
        let validity = if host.due_by.is_some() { "valid" } else { "invalid" };
        let repeats_str = if host.repeats { "repeats" } else { "one-shot" };
        let selector_str = host.selector.as_str(&env.mem);
        format!(
            "<NSTimer: {:?}; {} {}; interval={:.4}s; target={:?}; selector={}>",
            this,
            validity,
            repeats_str,
            host.ns_interval,
            host.target,
            selector_str
        )
    };
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

/// For use by `CADisplayLink`
pub fn set_time_interval(env: &mut Environment, timer: id, interval: NSTimeInterval) {
    let host_object = env.objc.borrow_mut::<NSTimerHostObject>(timer);
    let safe_interval = if interval.is_finite() && interval > 0.0001 {
        interval
    } else {
        0.0001
    };
    host_object.ns_interval = safe_interval;
    host_object.rust_interval = ns_time_interval_to_duration_or_zero(safe_interval);
}

/// For use by `NSRunLoop`
pub(super) fn set_run_loop(env: &mut Environment, timer: id, run_loop: id) {
    let host_object = env.objc.borrow_mut::<NSTimerHostObject>(timer);
    host_object.run_loop = run_loop;
}

/// For use by `NSRunLoop`: check if a timer is due to fire and fire it if
//necessary.
/// Returns the next firing time, if any.
pub(super) fn handle_timer(env: &mut Environment, timer: id) -> Option<Instant> {
    let &NSTimerHostObject {
        ns_interval,
        rust_interval,
        target,
        selector,
        invocation,
        repeats,
        due_by,
        is_running_callback,
        ..
    } = env.objc.borrow(timer);

    if is_running_callback {
        return None;
    }

    let due_by = due_by?;
    let now = Instant::now();

    if due_by > now {
        return Some(due_by);
    }

    let overdue_by = now.duration_since(due_by);
    retain(env, timer);

    if repeats {
        // Guard every step of the rescheduling math against junk floats. A
        // pathological ns_interval (e.g. NaN — see HyperHLE log #4) used
        // to propagate through `.ceil() as u32` (yielding 0 for NaN) and
        // then through `checked_mul`/`checked_add` until something
        // eventually unwrapped to a panic. Here we keep the computation
        // strictly bounded and fall back to the next interval if any step
        // overflows.
        let ratio = if ns_interval.is_finite() && ns_interval > 0.0 {
            (overdue_by.as_secs_f64() / ns_interval).max(1.0).ceil()
        } else {
            1.0
        };
        let advance_by: u32 = if ratio.is_finite() && (1.0..=(u32::MAX as f64)).contains(&ratio) {
            ratio as u32
        } else {
            1
        };
        let advance_by_dur = rust_interval
            .checked_mul(advance_by)
            .unwrap_or(rust_interval);
        let next_time = due_by
            .checked_add(advance_by_dur)
            .unwrap_or_else(|| Instant::now() + rust_interval);
        env.objc.borrow_mut::<NSTimerHostObject>(timer).due_by = Some(next_time);
    }

    env.objc
        .borrow_mut::<NSTimerHostObject>(timer)
        .is_running_callback = true;

    log_once!("First NSTimer fired (run loop is delivering scheduled events)");

    log_dbg!(
        "Timer {:?} fired, sending {:?} message to {:?}",
        timer,
        selector.as_str(&env.mem),
        target
    );

    let pool: id = msg_class![env; NSAutoreleasePool new];
    if invocation != nil {
        let _: () = msg![env; invocation invoke];
    } else {
        let _: () = msg_send(env, (target, selector, timer));
    }
    release(env, pool);

    env.objc
        .borrow_mut::<NSTimerHostObject>(timer)
        .is_running_callback = false;

    if !repeats {
        let _: () = msg![env; timer invalidate];
    }

    let final_due_by = env.objc.borrow::<NSTimerHostObject>(timer).due_by;
    release(env, timer);

    final_due_by
}
