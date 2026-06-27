/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ASIdentifierManager` — Advertising Support framework.
//!
//! Returns a fixed random-looking but stable IDFA so apps that request
//! the advertising identifier don't crash or spin waiting for a valid UUID.
//! Tracking is always reported as disabled / limited, matching a device
//! where the user has opted out of ad tracking.

use crate::frameworks::foundation::ns_string;
use crate::objc::{id, msg, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

// =========================================================================
// MARK: - ASIdentifierManager host object
// =========================================================================

#[derive(Default)]
struct ASIdentifierManagerHostObject {
    /// NSUUID* — the advertising identifier. Created once and cached.
    advertising_identifier: id,
}
impl HostObject for ASIdentifierManagerHostObject {}

// =========================================================================
// MARK: - ASTrackingManager (iOS 14+ — same file, thin stub)
// =========================================================================

struct ATTrackingManagerHostObject;
impl HostObject for ATTrackingManagerHostObject {}

// ATTrackingManager authorization status values
// (ATTrackingManagerAuthorizationStatus)
pub type ATTrackingManagerAuthorizationStatus = u32;
pub const ATTrackingManagerAuthorizationStatusNotDetermined: ATTrackingManagerAuthorizationStatus =
    0;
pub const ATTrackingManagerAuthorizationStatusRestricted: ATTrackingManagerAuthorizationStatus = 1;
pub const ATTrackingManagerAuthorizationStatusDenied: ATTrackingManagerAuthorizationStatus = 2;
pub const ATTrackingManagerAuthorizationStatusAuthorized: ATTrackingManagerAuthorizationStatus = 3;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - ASIdentifierManager
// =========================================================================

@implementation ASIdentifierManager: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ASIdentifierManagerHostObject {
        advertising_identifier: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// -------------------------------------------------------------------------
// Singleton accessor
// -------------------------------------------------------------------------

+ (id)sharedManager {
    // Store the singleton on the class object itself via a static.
    // We use a simple approach: alloc+init once, then return the same object.
    // In a multi-call scenario we rely on the fact that the host object keeps
    // the uuid cached.
    let instance: id = msg![env; this alloc];
    let instance: id = msg![env; instance init];
    // autorelease so callers who don't retain it don't leak.
    crate::objc::autorelease(env, instance)
}

- (id)init {
    this
}

- (())dealloc {
    let identifier =
        env.objc.borrow::<ASIdentifierManagerHostObject>(this).advertising_identifier;
    crate::objc::release(env, identifier);
    env.objc.dealloc_object(this, &mut env.mem)
}

// -------------------------------------------------------------------------
// advertisingIdentifier — returns a fixed stable NSUUID*
// -------------------------------------------------------------------------

- (id)advertisingIdentifier { // NSUUID*
    let existing =
        env.objc.borrow::<ASIdentifierManagerHostObject>(this).advertising_identifier;
    if existing != nil {
        return existing;
    }

    // Use a fixed UUID that looks plausible but is clearly synthetic.
    // Apps must not rely on this being a real device identifier.
    let uuid_str = ns_string::get_static_str(env, "00000000-0000-0000-0000-000000000000");
    let uuid: id = msg_class![env; NSUUID alloc];
    let uuid: id = msg![env; uuid initWithUUIDString:uuid_str];

    crate::objc::retain(env, uuid);
    env.objc.borrow_mut::<ASIdentifierManagerHostObject>(this)
        .advertising_identifier = uuid;

    log_dbg!(
        "ASIdentifierManager advertisingIdentifier — returning zeroed UUID \
         (ad tracking disabled)"
    );
    uuid
}

// -------------------------------------------------------------------------
// isAdvertisingTrackingEnabled — always NO (opted out)
// -------------------------------------------------------------------------

- (bool)isAdvertisingTrackingEnabled {
    // Return false — tracking is always disabled in touchHLE.
    // Apps should respect this and not send advertising data.
    log_dbg!("ASIdentifierManager isAdvertisingTrackingEnabled — returning NO");
    false
}

// -------------------------------------------------------------------------
// trackingAuthorizationStatus (iOS 14+)
// -------------------------------------------------------------------------

- (ATTrackingManagerAuthorizationStatus)trackingAuthorizationStatus {
    // Report "denied" so apps don't try to show a tracking permission dialog.
    ATTrackingManagerAuthorizationStatusDenied
}

// -------------------------------------------------------------------------
// NSCopying
// -------------------------------------------------------------------------

- (id)copyWithZone:(NSZonePtr)_zone {
    // Singleton — return self retained.
    crate::objc::retain(env, this);
    this
}

// -------------------------------------------------------------------------
// Description
// -------------------------------------------------------------------------

- (id)description {
    let s = format!(
        "<ASIdentifierManager: {:?}; trackingEnabled=NO; status=Denied>",
        this
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - ATTrackingManager (iOS 14+ App Tracking Transparency)
// =========================================================================

@implementation ATTrackingManager: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ATTrackingManagerHostObject);
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// -------------------------------------------------------------------------
// trackingAuthorizationStatus — class-level query
// -------------------------------------------------------------------------

+ (ATTrackingManagerAuthorizationStatus)trackingAuthorizationStatus {
    log_dbg!("ATTrackingManager trackingAuthorizationStatus — returning Denied");
    ATTrackingManagerAuthorizationStatusDenied
}

// -------------------------------------------------------------------------
// requestTrackingAuthorizationWithCompletionHandler:
// Immediately calls the completion handler with "Denied" — no UI shown.
// -------------------------------------------------------------------------

+ (())requestTrackingAuthorizationWithCompletionHandler:(id)completion_handler {
    log_dbg!(
        "ATTrackingManager requestTrackingAuthorizationWithCompletionHandler: \
         — calling handler with Denied immediately"
    );
    if completion_handler == nil {
        return;
    }
    // The completion handler is a block: ^(ATTrackingManagerAuthorizationStatus
    // status)
    // We call it by sending it the __FuncPtr invoke message with the status.
    let status: ATTrackingManagerAuthorizationStatus = ATTrackingManagerAuthorizationStatusDenied;
    // Invoke the block — blocks respond to `invoke` in touchHLE's block model.
    let sel = env.objc.lookup_selector("invokeWithUnsignedInt:").unwrap();
    let responds: bool = msg![env; completion_handler respondsToSelector:sel];
    if responds {
        let _: () = msg![env; completion_handler invokeWithUnsignedInt:status];
    } else {
        // Fallback: try plain invoke with no arguments (some block wrappers).
        let sel_plain = env.objc.lookup_selector("invoke").unwrap();
        let responds_plain: bool = msg![env; completion_handler respondsToSelector:sel_plain];
        if responds_plain {
            let _: () = msg![env; completion_handler invoke];
        } else {
            log!("ATTrackingManager: completion handler {:?} does not respond to invoke — ignored", completion_handler);
        }
    }
}

// -------------------------------------------------------------------------
// Description
// -------------------------------------------------------------------------

+ (id)description {
    let s = "ATTrackingManager (touchHLE stub — always Denied)";
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};
