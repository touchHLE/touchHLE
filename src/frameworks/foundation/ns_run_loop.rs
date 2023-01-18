//! `NSRunLoop`.
//!
//! Resources:
//! - Apple's [Threading Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/Introduction/Introduction.html)

use super::{ns_string, ns_timer};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::audio_toolbox::audio_queue::AudioQueueRef;
use crate::frameworks::core_foundation::cf_run_loop::{
    kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoopRef,
};
use crate::objc::{id, msg, objc_classes, retain, ClassExports, HostObject};
use crate::window::Event;
use crate::Environment;

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
        "_NSRunLoopDefaultMode",
        HostConstant::NSString(NSDefaultRunLoopMode),
    ),
];

#[derive(Default)]
pub struct State {
    main_thread_run_loop: Option<id>,
}

struct NSRunLoopHostObject {
    /// Weak reference. Audio queue must remove itself when destroyed (TODO).
    audio_queues: Vec<AudioQueueRef>,
    /// Strong references to `NSTimer*`. Timers are owned by the run loop.
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
    // TODO: handle other modes
    assert!(msg![env; mode isEqualToString:default_mode]);

    log_dbg!("Adding timer {:?} to run loop {:?}", timer, this);

    retain(env, timer);

    let host_object = env.objc.borrow_mut::<NSRunLoopHostObject>(this);
    assert!(!host_object.timers.contains(&timer)); // TODO: what do we do here?
    host_object.timers.push(timer);
    ns_timer::set_run_loop(env, timer, this);
}

- (())run {
    run_run_loop(env, this);
}
// TODO: other run methods

@end

};

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

fn run_run_loop(env: &mut Environment, run_loop: id) {
    log_dbg!("Entering run loop {:?} (indefinitely)", run_loop);

    loop {
        env.window.poll_for_events();

        while let Some(event) = env.window.pop_event() {
            // FIXME: tell the app when we're about to quit
            let Event::Quit = event;
            panic!("User requested quit, exiting.");
        }

        // TODO: handle timers and audio queues
    }
}
