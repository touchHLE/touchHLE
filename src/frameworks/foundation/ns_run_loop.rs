/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSRunLoop`.
//!
//! Resources:
//! - Apple's [Threading Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/Introduction/Introduction.html)

use super::{ns_string, ns_timer, NSTimeInterval};
use crate::dyld::{ConstantExports, HostConstant};
use crate::environment::ThreadId;
use crate::frameworks::audio_toolbox::audio_queue::{handle_audio_queue, AudioQueueRef};
use crate::frameworks::audio_toolbox::audio_unit::{render_audio_unit, AudioUnit};
use crate::frameworks::core_foundation::cf_run_loop::{
    kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoopRef,
};
use crate::frameworks::{core_animation, media_player, uikit};
use crate::objc::{id, msg, objc_classes, release, retain, Class, ClassExports, HostObject};
use crate::Environment;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// `NSString*`
pub type NSRunLoopMode = id;
// FIXME: Maybe this shouldn't be the same value? See: https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/RunLoopManagement/RunLoopManagement.html
pub const NSRunLoopCommonModes: &str = kCFRunLoopCommonModes;
pub const NSDefaultRunLoopMode: &str = kCFRunLoopDefaultMode;

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSRunLoopCommonModes",
        HostConstant::NSString(NSRunLoopCommonModes),
    ),
    (
        "_NSDefaultRunLoopMode",
        HostConstant::NSString(NSDefaultRunLoopMode),
    ),
];

#[derive(Default)]
pub struct State {
    run_loops: HashMap<ThreadId, id>,
}

struct NSRunLoopHostObject {
    audio_units: Vec<AudioUnit>,
    /// Weak reference. Audio queue must remove itself when destroyed (TODO).
    /// They are in no particular order.
    audio_queues: Vec<AudioQueueRef>,
    /// Strong references to `NSTimer*` in no particular order. Timers are owned
    /// by the run loop. The timer must remove itself when invalidated.
    timers: Vec<id>,
    /// A bool flag to indicate if the run loop is running.
    /// It is needed to deal with re-entrance issues.
    is_running: bool,
}
impl HostObject for NSRunLoopHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSRunLoop: NSObject

+ (id)mainRunLoop {
    run_loop_for_thread(env, this, 0)
}

+ (id)currentRunLoop {
    run_loop_for_thread(env, this, env.current_thread)
}

// TODO: more accessors

- (id) retain { this }
- (()) release {}
- (id) autorelease { this }

- (CFRunLoopRef)getCFRunLoop {
    // In our implementation these are the same type (they aren't in Apple's).
    this
}

- (())addTimer:(id)timer // NSTimer*
       forMode:(NSRunLoopMode)mode {
    let default_mode = ns_string::get_static_str(env, NSDefaultRunLoopMode);
    let common_modes = ns_string::get_static_str(env, NSRunLoopCommonModes);
    // TODO: handle other modes
    assert!(msg![env; mode isEqualToString:default_mode] || msg![env; mode isEqualToString:common_modes]);

    log_dbg!(
        "Adding timer {:?} to run loop {:?} with mode {:?}",
        timer,
        this,
        ns_string::to_rust_string(env, mode),
    );

    retain(env, timer);

    let host_object = env.objc.borrow_mut::<NSRunLoopHostObject>(this);
    assert!(!host_object.timers.contains(&timer)); // TODO: what do we do here?
    host_object.timers.push(timer);
    ns_timer::set_run_loop(env, timer, this);
}

- (())run {
    run_run_loop(env, this, /* single_iteration: */ false, None);
}

- (())runUntilDate:(id)date {
    let time_limit: NSTimeInterval = msg![env; date timeIntervalSince1970];
    run_run_loop(env, this, /* single_iteration: */ false, Some(time_limit));
}

// TODO: other run methods

@end

};

/// For use by Audio Toolbox.
pub fn add_audio_unit(env: &mut Environment, run_loop: id, unit: AudioUnit) {
    env.objc
        .borrow_mut::<NSRunLoopHostObject>(run_loop)
        .audio_units
        .push(unit);
}

/// For use by Audio Toolbox.
pub fn remove_audio_unit(env: &mut Environment, run_loop: id, unit: AudioUnit) {
    let units = &mut env
        .objc
        .borrow_mut::<NSRunLoopHostObject>(run_loop)
        .audio_units;
    let unit_idx = units.iter().position(|&item| item == unit).unwrap();
    units.remove(unit_idx);
}

/// For use by Audio Toolbox.
/// TODO: Maybe replace this with a `CFRunLoopObserver` or some other generic
/// mechanism?
/// TODO: Handle run loop modes. Currently assumes the common modes.
pub fn add_audio_queue(env: &mut Environment, run_loop: id, queue: AudioQueueRef) {
    env.objc
        .borrow_mut::<NSRunLoopHostObject>(run_loop)
        .audio_queues
        .push(queue);
}

/// For use by Audio Toolbox.
pub fn remove_audio_queue(env: &mut Environment, run_loop: id, queue: AudioQueueRef) {
    let queues = &mut env
        .objc
        .borrow_mut::<NSRunLoopHostObject>(run_loop)
        .audio_queues;
    let queue_idx = queues.iter().position(|&item| item == queue).unwrap();
    queues.remove(queue_idx);
}

/// For use by NSTimer so it can remove itself once it's invalidated.
pub(super) fn remove_timer(env: &mut Environment, run_loop: id, timer: id) {
    let NSRunLoopHostObject { timers, .. } = env.objc.borrow_mut(run_loop);

    let mut i = 0;
    let mut release_count = 0;
    while i < timers.len() {
        if timers[i] == timer {
            timers.swap_remove(i);
            release_count += 1;
        } else {
            i += 1;
        }
    }
    assert!(release_count == 1); // TODO?
    for _ in 0..release_count {
        release(env, timer);
    }
}

/// Run the run loop for just a single iteration. This is a special mode just
/// for the app picker, since we don't have `runMode:beforeDate:` yet.
/// (TODO: implement those to replace this.)
pub fn run_run_loop_single_iteration(env: &mut Environment, run_loop: id) {
    run_run_loop(env, run_loop, /* single_iteration: */ true, None)
}

pub fn run_run_loop(
    env: &mut Environment,
    run_loop: id,
    single_iteration: bool,
    unix_time_limit: Option<f64>,
) {
    if single_iteration {
        log_dbg!(
            "Entering run loop {:?} (single iteration), limit {:?}",
            run_loop,
            unix_time_limit
        );
    } else {
        log_dbg!(
            "Entering run loop {:?} (indefinitely), limit {:?}",
            run_loop,
            unix_time_limit
        );
    }

    if env.objc.borrow::<NSRunLoopHostObject>(run_loop).is_running {
        // TODO: The code right now can't handle re-entrancy properly; a timer
        //       callback that re-enters the run loop will cause an infite loop.
        //       This needs to be fixed. For now, we skip execution to avoid
        //       triggering these bugs, but this means the app can't yield
        //       control. :(
        log_dbg!(
            "Run loop {:?} is already running, skipping (TODO: support run loop re-entrancy)",
            run_loop
        );
        log_once!(
            "Warning: run loop re-entrancy is unimplemented but may be relied upon by this app."
        );
        return;
    };
    env.objc
        .borrow_mut::<NSRunLoopHostObject>(run_loop)
        .is_running = true;

    // Temporary vectors used to track things without needing a reference to the
    // environment or to lock the object. Re-used each iteration for efficiency.
    let mut timers_tmp = Vec::new();
    let mut audio_queues_tmp = Vec::new();
    let mut audio_units_tmp = Vec::new();

    fn limit_sleep_time(current: &mut Option<Instant>, new: Option<Instant>) {
        if let Some(new) = new {
            *current = Some(current.map_or(new, |i| i.min(new)));
        }
    }

    let is_main_run_loop = env.current_thread == 0;

    loop {
        let mut sleep_until = None;

        // We want to process those only on the main run loop
        if is_main_run_loop {
            env.window
                .as_mut()
                .expect("NSRunLoop not supported in headless mode")
                .poll_for_events(&env.options);

            let next_due = uikit::handle_events(env);
            limit_sleep_time(&mut sleep_until, next_due);

            let next_due = core_animation::recomposite_if_necessary(env, /* force: */ false);
            limit_sleep_time(&mut sleep_until, next_due);
        }

        assert!(timers_tmp.is_empty());
        timers_tmp.extend_from_slice(&env.objc.borrow::<NSRunLoopHostObject>(run_loop).timers);

        for timer in timers_tmp.drain(..) {
            let next_due = ns_timer::handle_timer(env, timer);
            limit_sleep_time(&mut sleep_until, next_due);
        }

        assert!(audio_queues_tmp.is_empty());
        audio_queues_tmp.extend_from_slice(
            &env.objc
                .borrow::<NSRunLoopHostObject>(run_loop)
                .audio_queues,
        );

        for audio_queue in audio_queues_tmp.drain(..) {
            handle_audio_queue(env, audio_queue);
        }

        // TODO: not clear if audio units should be processed in the run loop
        assert!(audio_units_tmp.is_empty());
        audio_units_tmp
            .extend_from_slice(&env.objc.borrow::<NSRunLoopHostObject>(run_loop).audio_units);

        for audio_unit in audio_units_tmp.drain(..) {
            render_audio_unit(env, audio_unit);
        }

        if is_main_run_loop {
            media_player::handle_players(env);
        }

        // Unfortunately, touchHLE has to poll for certain things repeatedly;
        // it can't just wait until the next event appears.
        //
        // For optimal responsiveness we could poll as often as possible, but
        // this results in 100% usage of a CPU core and excessive energy use.
        // On the other hand, for optimal energy use we could always sleep until
        // the next scheduled event (e.g. the next timer), but this would lead
        // to late handling of unscheduled events (e.g. a finger movement) and
        // events that are scheduled but we can't get the time for currently
        // (audio queue buffer exhaustion).
        //
        // The compromise used here is that we will wait for a 60th of a second,
        // or until the next scheduled event, whichever is sooner. iPhone OS
        // apps can't do more than 60fps so this should be fine.
        let limit = Duration::from_millis(1000 / 60);
        env.sleep(
            sleep_until.map_or(limit, |i| i.duration_since(Instant::now()).min(limit)),
            false,
        );

        if single_iteration {
            break;
        }

        if let Some(limit) = unix_time_limit {
            // We use Unix epoch as a convenience reference date.
            // (Apple's epoch is less convenient in Rust. And "pure"
            // Rust approach with Duration/Instant is just too troublesome
            // and not worthy to convert back and forth)
            if SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                >= limit
            {
                break;
            }
        }
    }

    env.objc
        .borrow_mut::<NSRunLoopHostObject>(run_loop)
        .is_running = false;
}

/// Helper method for `mainRunLoop` and `currentRunLoop` NSThread class methods
fn run_loop_for_thread(env: &mut Environment, this: Class, thread_id: ThreadId) -> id {
    if let std::collections::hash_map::Entry::Vacant(e) = env
        .framework_state
        .foundation
        .ns_run_loop
        .run_loops
        .entry(thread_id)
    {
        let host_object = Box::new(NSRunLoopHostObject {
            audio_units: Vec::new(),
            audio_queues: Vec::new(),
            timers: Vec::new(),
            is_running: false,
        });
        // TODO: is it OK to allocate static object for all threads,
        // not only main one?
        let new = env
            .objc
            .alloc_static_object(this, host_object, &mut env.mem);
        e.insert(new);
    }
    *env.framework_state
        .foundation
        .ns_run_loop
        .run_loops
        .get(&thread_id)
        .unwrap()
}
