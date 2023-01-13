//! `NSRunLoop`.

use crate::frameworks::audio_toolbox::audio_queue::AudioQueueRef;
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopRef;
use crate::objc::{id, msg, objc_classes, ClassExports, HostObject};
use crate::Environment;

#[derive(Default)]
pub struct State {
    main_thread_run_loop: Option<id>,
}

struct NSRunLoopHostObject {
    /// Weak reference. Audio queue must remove itself when destroyed (TODO).
    audio_queues: Vec<AudioQueueRef>,
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
