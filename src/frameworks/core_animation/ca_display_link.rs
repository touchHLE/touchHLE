/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CADisplayLink`

use crate::frameworks::foundation::ns_run_loop::{self, NSRunLoopMode};
use crate::frameworks::foundation::NSInteger;
use crate::msg;
use crate::objc::{
    autorelease, id, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr, SEL,
};

#[derive(Default)]
struct CADisplayLinkHostObject {
    target: id,
    selector: Option<SEL>,
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
    let host_object = env.objc.borrow_mut::<CADisplayLinkHostObject>(display_link);
    host_object.target = target;
    host_object.selector = Some(sel);
    retain(env, target);
    log_dbg!("[CADisplayLink displayLinkWithTarget:{:?} selector:{}] => {:?}", target, sel.as_str(&env.mem), display_link);
    autorelease(env, display_link)
}

- (())setFrameInterval:(NSInteger)frameInterval {
    log!("TODO: [(CADisplayLink*){:?} setFrameInterval:{}]", this, frameInterval);
}

- (())addToRunLoop:(id)run_loop forMode:(NSRunLoopMode)mode {
    log_dbg!("[(CADisplayLink*){:?} addToRunLoop:{:?} forMode:{:?}]", this, run_loop, mode);
    ns_run_loop::add_display_link_for_mode(env, run_loop, this, mode);
}

- (())removeFromRunLoop:(id)run_loop forMode:(NSRunLoopMode)mode {
    log_dbg!("[(CADisplayLink*){:?} removeFromRunLoop:{:?} forMode:{:?}]", this, run_loop, mode);
    ns_run_loop::remove_display_link_for_mode(env, run_loop, this, mode);
}

- (())invalidate {
    log_dbg!("[(CADisplayLink*){:?} invalidate]", this);
    ns_run_loop::remove_display_link_from_all_run_loops(env, this);
}

- (())dealloc {
    let host_object = env.objc.borrow::<CADisplayLinkHostObject>(this);
    release(env, host_object.target);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

pub fn trigger_display_link(env: &mut crate::Environment, display_link: id) {
    let CADisplayLinkHostObject { target, selector } =
        *env.objc.borrow::<CADisplayLinkHostObject>(display_link);
    let selector = selector.unwrap();
    log_dbg!(
        "CADisplayLink triggering [{:?} {}]",
        target,
        selector.as_str(&env.mem)
    );
    let _: id = msg![env; target performSelector:selector withObject:display_link];
}
