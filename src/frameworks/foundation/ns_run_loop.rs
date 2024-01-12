/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSRunLoop`.
//!
//! Resources:
//! - Apple's [Threading Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/Introduction/Introduction.html)

use super::{ns_string, ns_timer};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::audio_toolbox::audio_queue::{handle_audio_queue, AudioQueueRef};
use crate::frameworks::audio_toolbox::audio_unit::{render_audio_unit, AudioUnit};
use crate::frameworks::core_foundation::cf_run_loop::{
    kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoopRef,
};
use crate::frameworks::{core_animation, media_player, uikit};
use crate::objc::{id, msg, objc_classes, release, retain, ClassExports, HostObject};
use crate::Environment;
use std::time::{Duration, Instant};

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
    main_thread_run_loop: Option<id>,
}

struct NSRunLoopHostObject {
    audio_units: Vec<AudioUnit>,
    /// Weak reference. Audio queue must remove itself when destroyed (TODO).
    /// They are in no particular order.
    audio_queues: Vec<AudioQueueRef>,
    /// Strong references to `NSTimer*` in no particular order. Timers are owned
    /// by the run loop. The timer must remove itself when invalidated.
    timers: Vec<id>,
}
impl HostObject for NSRunLoopHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSRunLoop: NSObject

+ (id)mainRunLoop {
    if let Some(rl) = env.framework_state.foundation.ns_run_loop.main_thread_run_loop {
        rl
    } else {
        let host_object = Box::new(NSRunLoopHostObject {
            audio_units: Vec::new(),
            audio_queues: Vec::new(),
            timers: Vec::new(),
        });
        let new = env.objc.alloc_static_object(this, host_object, &mut env.mem);
        env.framework_state.foundation.ns_run_loop.main_thread_run_loop = Some(new);
        new
    }
}

+ (id)currentRunLoop {
    assert!(env.current_thread == 0);
    msg![env; this mainRunLoop]
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
    run_run_loop(env, this, /* single_iteration: */ false);
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
/// for the app picker, since we don't have `runMode:beforeDate:` or
/// `runUntilDate:` yet. (TODO: implement those to replace this.)
pub fn run_run_loop_single_iteration(env: &mut Environment, run_loop: id) {
    run_run_loop(env, run_loop, /* single_iteration: */ true)
}

fn run_run_loop(env: &mut Environment, run_loop: id, single_iteration: bool) {
    if single_iteration {
        log_dbg!("Entering run loop {:?} (single iteration)", run_loop);
    } else {
        log_dbg!("Entering run loop {:?} (indefinitely)", run_loop);
    }

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

    loop {
        let mut sleep_until = None;

        env.window
            .as_mut()
            .expect("NSRunLoop not supported in headless mode")
            .poll_for_events(&env.options);

        let next_due = uikit::handle_events(env);
        limit_sleep_time(&mut sleep_until, next_due);

        let next_due = core_animation::recomposite_if_necessary(env);
        limit_sleep_time(&mut sleep_until, next_due);

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

        assert!(audio_units_tmp.is_empty());
        audio_units_tmp
            .extend_from_slice(&env.objc.borrow::<NSRunLoopHostObject>(run_loop).audio_units);

        for audio_unit in audio_units_tmp.drain(..) {
            render_audio_unit(env, audio_unit);
        }

        media_player::handle_players(env);

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
    }
}
