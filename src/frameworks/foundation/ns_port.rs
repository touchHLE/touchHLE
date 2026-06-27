/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSPort` and `NSMachPort`.
//!
//! Resources:
//! - Apple's NSPort documentation:
//!   <https://developer.apple.com/documentation/foundation/nsport>
//! - Apple's NSMachPort documentation:
//!   <https://developer.apple.com/documentation/foundation/nsmachport>
//!
//! Per Apple's documentation, `+[NSPort port]` / `+[NSMachPort port]` create
//! a new port object, and `+[NSPort allocWithZone:]` actually "returns an
//! instance of the NSMachPort class" on Cocoa. Ports are used for
//! inter-thread/inter-process messaging via the run loop; the most common
//! pattern in old iPhone OS games (e.g. EA's Tetris) is creating a port and
//! attaching it to a run loop just to keep `-[NSRunLoop run]` from returning
//! immediately:
//!
//! ```objc
//! [[NSRunLoop currentRunLoop] addPort:[NSMachPort port] forMode:NSDefaultRunLoopMode];
//! ```
//!
//! touchHLE has no Mach kernel, so there is no real Mach port to wrap.
//! What apps actually rely on is:
//! - `+port`/`+new`/`alloc`+`init` returning a real, retainable object
//!   (previously this returned nil, and `[NSMachPort new]` logged
//!   "Class NSMachPort is unimplemented" — the nil then propagated into
//!   `-[NSRunLoop addPort:forMode:]` and the app's worker thread setup
//!   broke, as seen with TetrisApp).
//! - `scheduleInRunLoop:forMode:` / `removeFromRunLoop:forMode:` being
//!   callable.
//! - `isValid` returning true until `invalidate` is called, matching the
//!   documented NSPort contract.

use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

#[derive(Default)]
struct NSMachPortHostObject {
    /// Per Apple's NSPort docs, a port is valid from creation until
    /// `invalidate` is called.
    valid: bool,
    /// `id<NSMachPortDelegate>` — not retained, per Apple's `setDelegate:`
    /// contract ("the receiver does not retain its delegate").
    delegate: id,
    /// The wrapped Mach port right (always a fake value here).
    mach_port: u32,
}
impl HostObject for NSMachPortHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSPort is documented as a class cluster whose +allocWithZone: returns an
// NSMachPort instance on Cocoa, so we implement everything on NSMachPort and
// make NSPort forward to it.
@implementation NSPort: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // Per Apple's NSPort documentation: "allocWithZone: returns an instance
    // of the NSMachPort class" when sent to the NSPort class.
    let mach_port_class: crate::objc::Class = msg_class![env; NSMachPort class];
    msg![env; mach_port_class allocWithZone:zone]
}

+ (id)port {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    autorelease(env, new)
}

@end

@implementation NSMachPort: NSPort

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSMachPortHostObject {
        valid: true,
        delegate: nil,
        mach_port: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)port {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    autorelease(env, new)
}

// `+[NSMachPort portWithMachPort:]` per Apple's docs: creates and returns
// a port object configured with the given Mach port right.
+ (id)portWithMachPort:(u32)mach_port {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithMachPort:mach_port];
    autorelease(env, new)
}

- (id)init {
    env.objc.borrow_mut::<NSMachPortHostObject>(this).valid = true;
    this
}

- (id)initWithMachPort:(u32)mach_port {
    let host = env.objc.borrow_mut::<NSMachPortHostObject>(this);
    host.valid = true;
    host.mach_port = mach_port;
    this
}

- (u32)machPort {
    env.objc.borrow::<NSMachPortHostObject>(this).mach_port
}

- (bool)isValid {
    env.objc.borrow::<NSMachPortHostObject>(this).valid
}

// Per Apple's NSPort docs, invalidate marks the port as invalid and posts
// NSPortDidBecomeInvalidNotification. We skip the notification (no
// observers can exist for a port that never receives real messages).
- (())invalidate {
    env.objc.borrow_mut::<NSMachPortHostObject>(this).valid = false;
}

- (id)delegate {
    env.objc.borrow::<NSMachPortHostObject>(this).delegate
}

// Per Apple's setDelegate: contract, the delegate is NOT retained.
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<NSMachPortHostObject>(this).delegate = delegate;
}

// Run-loop scheduling. touchHLE's run loop has no port-message sources;
// these only need to be safely callable. The retain/release pair mirrors
// the ownership the real run loop would take on the scheduled port.
- (())scheduleInRunLoop:(id)_run_loop forMode:(id)_mode {
    log_dbg!("[(NSMachPort*){:?} scheduleInRunLoop:forMode:] (no-op)", this);
}

- (())removeFromRunLoop:(id)_run_loop forMode:(id)_mode {
    log_dbg!("[(NSMachPort*){:?} removeFromRunLoop:forMode:] (no-op)", this);
}

// Per Apple's NSPort docs (reservedSpaceLength): "The default length is 0."
- (u32)reservedSpaceLength {
    0
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

/// For use by `NSRunLoop`'s `addPort:forMode:` / `removePort:forMode:`:
/// retain/release semantics helper (the run loop owns scheduled ports).
pub fn retain_port(env: &mut crate::Environment, port: id) {
    if port != nil {
        retain(env, port);
    }
}
pub fn release_port(env: &mut crate::Environment, port: id) {
    if port != nil {
        release(env, port);
    }
}
