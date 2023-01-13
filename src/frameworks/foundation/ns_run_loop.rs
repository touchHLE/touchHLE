//! `NSRunLoop`.

use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopRef;
use crate::objc::{id, msg, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    main_thread_run_loop: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSRunLoop: NSObject

+ (id)mainRunLoop {
    if let Some(rl) = env.framework_state.foundation.ns_run_loop.main_thread_run_loop {
        rl
    } else {
        let new: id = msg![env; this alloc];
        let new: id = msg![env; new init];
        env.framework_state.foundation.ns_run_loop.main_thread_run_loop = Some(new);
        new
    }
}

+ (id)currentRunLoop {
    assert!(env.current_thread == 0);
    msg![env; this mainRunLoop]
}

// TODO: more accessors

- (CFRunLoopRef)getCFRunLoop {
    // In our implementation these are the same type (they aren't in Apple's).
    this
}

@end

};
