/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSProcessInfo`.

use super::NSTimeInterval;
use crate::frameworks::foundation::ns_string;
use crate::libc::mach::host::PHYSICAL_MEMORY;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports};
use crate::Environment;
use std::time::Instant;

#[derive(Default)]
pub struct State {
    /// `NSProcessInfo*`
    process_info: Option<id>,
}

fn assert_process_info_singleton(env: &mut Environment, this: id) {
    assert_eq!(
        this,
        env.framework_state
            .foundation
            .ns_process_info
            .process_info
            .unwrap()
    );
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

+ (id)processInfo {
    if let Some(existing) = env.framework_state.foundation.ns_process_info.process_info {
        existing
    } else {
        let process_info: id = msg![env; this new];
        env.framework_state.foundation.ns_process_info.process_info = Some(process_info);
        process_info
    }
}

- (NSTimeInterval)systemUptime {
    assert_process_info_singleton(env, this); // TODO
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

- (u64)physicalMemory {
    assert_process_info_singleton(env, this); // TODO
    PHYSICAL_MEMORY.into()
}

- (id)processName {
    // This function probably just needs to return a unique value
    // Testing on macOS appears CFBundleName is used
    assert_process_info_singleton(env, this); // TODO
    let main_bundle: id = msg_class![env; NSBundle mainBundle];
    let name_key: id = ns_string::get_static_str(env, "CFBundleName");
    msg![env; main_bundle objectForInfoDictionaryKey:name_key]
}

@end

};
