/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ChipmunkHastySpace` — a subclass of `ChipmunkSpace` that uses multiple
//! threads for simulation. Since touchHLE is single-threaded we just treat it
//! identically to `ChipmunkSpace`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// =========================================================================
// MARK: - ChipmunkSpace host object (base)
// =========================================================================

pub struct ChipmunkSpaceHostObject {
    /// cpSpace* equivalent — we store bodies/shapes/constraints as guest ids.
    pub gravity_x: f32,
    pub gravity_y: f32,
    pub damping: f32,
    pub iterations: i32,
    pub sleep_time_threshold: f32,
    pub idle_speed_threshold: f32,
    pub collision_slop: f32,
    pub collision_bias: f32,
    pub collision_persistence: u32,
    pub static_body: id,
    pub delegate: id,
}
impl HostObject for ChipmunkSpaceHostObject {}

impl Default for ChipmunkSpaceHostObject {
    fn default() -> Self {
        ChipmunkSpaceHostObject {
            gravity_x: 0.0,
            gravity_y: 0.0,
            damping: 1.0,
            iterations: 10,
            sleep_time_threshold: f32::INFINITY,
            idle_speed_threshold: 0.0,
            collision_slop: 0.1,
            collision_bias: 0.0017,
            collision_persistence: 3,
            static_body: nil,
            delegate: nil,
        }
    }
}

// =========================================================================
// MARK: - ChipmunkHastySpace host object
// =========================================================================

pub struct ChipmunkHastySpaceHostObject {
    pub superclass: ChipmunkSpaceHostObject,
    /// Number of threads (ignored — we are single-threaded).
    pub threads: NSInteger,
}
impl HostObject for ChipmunkHastySpaceHostObject {}

impl Default for ChipmunkHastySpaceHostObject {
    fn default() -> Self {
        ChipmunkHastySpaceHostObject {
            superclass: ChipmunkSpaceHostObject::default(),
            threads: 1,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - ChipmunkSpace
// =========================================================================

@implementation ChipmunkSpace: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ChipmunkSpaceHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    log_dbg!("ChipmunkSpace init {:?}", this);
    this
}

- (())dealloc {
    let (static_body, delegate) = {
        let h = env.objc.borrow::<ChipmunkSpaceHostObject>(this);
        (h.static_body, h.delegate)
    };
    release(env, static_body);
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Gravity

- (())setGravity:(crate::frameworks::core_graphics::CGPoint)gravity {
    let host = env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this);
    host.gravity_x = gravity.x;
    host.gravity_y = gravity.y;
}

- (crate::frameworks::core_graphics::CGPoint)gravity {
    let h = env.objc.borrow::<ChipmunkSpaceHostObject>(this);
    crate::frameworks::core_graphics::CGPoint { x: h.gravity_x, y: h.gravity_y }
}

// MARK: Damping

- (f32)damping {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).damping
}
- (())setDamping:(f32)d {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).damping = d;
}

// MARK: Iterations

- (i32)iterations {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).iterations
}
- (())setIterations:(i32)i {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).iterations = i;
}

// MARK: Sleep / idle thresholds

- (f32)sleepTimeThreshold {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).sleep_time_threshold
}
- (())setSleepTimeThreshold:(f32)t {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).sleep_time_threshold = t;
}

- (f32)idleSpeedThreshold {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).idle_speed_threshold
}
- (())setIdleSpeedThreshold:(f32)t {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).idle_speed_threshold = t;
}

// MARK: Collision parameters

- (f32)collisionSlop {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).collision_slop
}
- (())setCollisionSlop:(f32)s {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).collision_slop = s;
}

- (f32)collisionBias {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).collision_bias
}
- (())setCollisionBias:(f32)b {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).collision_bias = b;
}

- (u32)collisionPersistence {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).collision_persistence
}
- (())setCollisionPersistence:(u32)p {
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).collision_persistence = p;
}

// MARK: Static body

- (id)staticBody {
    let existing = env.objc.borrow::<ChipmunkSpaceHostObject>(this).static_body;
    if existing != nil { return existing; }
    // Lazily create a static ChipmunkBody.
    let body: id = msg_class![env; ChipmunkBody alloc];
    let body: id = msg![env; body init];
    retain(env, body);
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).static_body = body;
    body
}

// MARK: Delegate

- (id)delegate {
    env.objc.borrow::<ChipmunkSpaceHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<ChipmunkSpaceHostObject>(this).delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<ChipmunkSpaceHostObject>(this).delegate = delegate;
}

// MARK: Add / remove objects

- (id)add:(id)obj {
    log_dbg!("ChipmunkSpace add:{:?} — stub", obj);
    obj
}

- (id)remove:(id)obj {
    log_dbg!("ChipmunkSpace remove:{:?} — stub", obj);
    obj
}

- (bool)contains:(id)obj {
    log_dbg!("ChipmunkSpace contains:{:?} — returning NO (stub)", obj);
    false
}

- (id)addShape:(id)shape {
    log_dbg!("ChipmunkSpace addShape:{:?} — stub", shape);
    shape
}

- (id)removeShape:(id)shape {
    log_dbg!("ChipmunkSpace removeShape:{:?} — stub", shape);
    shape
}

- (id)addBody:(id)body {
    log_dbg!("ChipmunkSpace addBody:{:?} — stub", body);
    body
}

- (id)removeBody:(id)body {
    log_dbg!("ChipmunkSpace removeBody:{:?} — stub", body);
    body
}

- (id)addConstraint:(id)constraint {
    log_dbg!("ChipmunkSpace addConstraint:{:?} — stub", constraint);
    constraint
}

- (id)removeConstraint:(id)constraint {
    log_dbg!("ChipmunkSpace removeConstraint:{:?} — stub", constraint);
    constraint
}

// MARK: Step

- (())step:(f32)dt {
    log_dbg!("ChipmunkSpace step:{} — stub (no physics simulation)", dt);
}

// MARK: Collision handlers

- (())addCollisionHandler:(id)_handler {
    log_dbg!("ChipmunkSpace addCollisionHandler: — stub");
}

- (())removeCollisionHandler:(id)_handler {
    log_dbg!("ChipmunkSpace removeCollisionHandler: — stub");
}

- (())setDefaultCollisionHandler:(id)_handler {
    log_dbg!("ChipmunkSpace setDefaultCollisionHandler: — stub");
}

// MARK: Queries

- (id)pointQueryAll:(crate::frameworks::core_graphics::CGPoint)_point
             layers:(u32)_layers
              group:(id)_group {
    // Return empty array — no shapes in our stub space.
    msg_class![env; NSArray array]
}

- (id)nearestPointQueryAll:(crate::frameworks::core_graphics::CGPoint)_point
                  maxDistance:(f32)_max_dist
                        layers:(u32)_layers
                         group:(id)_group {
    msg_class![env; NSArray array]
}

- (id)segmentQueryAll:(crate::frameworks::core_graphics::CGPoint)_start
                   to:(crate::frameworks::core_graphics::CGPoint)_end
               layers:(u32)_layers
                group:(id)_group {
    msg_class![env; NSArray array]
}

- (id)bbQuery:(id)_bb // ChipmunkBB
       layers:(u32)_layers
        group:(id)_group {
    msg_class![env; NSArray array]
}

// MARK: Description

- (id)description {
    let (gx, gy, iters) = {
        let h = env.objc.borrow::<ChipmunkSpaceHostObject>(this);
        (h.gravity_x, h.gravity_y, h.iterations)
    };
    let s = format!(
        "<ChipmunkSpace: {:?}; gravity=({},{}) iterations={}>",
        this, gx, gy, iters
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - ChipmunkHastySpace
// =========================================================================

@implementation ChipmunkHastySpace: ChipmunkSpace

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ChipmunkHastySpaceHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    log_dbg!("ChipmunkHastySpace init {:?}", this);
    this
}

- (id)initWithThreads:(NSInteger)threads {
    log_dbg!(
        "ChipmunkHastySpace initWithThreads:{} — single-threaded stub",
        threads
    );
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).threads = threads;
    this
}

// MARK: Threads property

- (NSInteger)threads {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).threads
}

- (())setThreads:(NSInteger)threads {
    log_dbg!(
        "ChipmunkHastySpace setThreads:{} — ignored (single-threaded)",
        threads
    );
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).threads = threads;
}

// MARK: Gravity / damping / iterations — forward to superclass fields

- (())setGravity:(crate::frameworks::core_graphics::CGPoint)gravity {
    let host = env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this);
    host.superclass.gravity_x = gravity.x;
    host.superclass.gravity_y = gravity.y;
}

- (crate::frameworks::core_graphics::CGPoint)gravity {
    let h = env.objc.borrow::<ChipmunkHastySpaceHostObject>(this);
    crate::frameworks::core_graphics::CGPoint {
        x: h.superclass.gravity_x,
        y: h.superclass.gravity_y,
    }
}

- (f32)damping {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.damping
}
- (())setDamping:(f32)d {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.damping = d;
}

- (i32)iterations {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.iterations
}
- (())setIterations:(i32)i {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.iterations = i;
}

- (f32)sleepTimeThreshold {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.sleep_time_threshold
}
- (())setSleepTimeThreshold:(f32)t {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.sleep_time_threshold = t;
}

- (f32)idleSpeedThreshold {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.idle_speed_threshold
}
- (())setIdleSpeedThreshold:(f32)t {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.idle_speed_threshold = t;
}

- (f32)collisionSlop {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.collision_slop
}
- (())setCollisionSlop:(f32)s {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.collision_slop = s;
}

- (f32)collisionBias {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.collision_bias
}
- (())setCollisionBias:(f32)b {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.collision_bias = b;
}

- (u32)collisionPersistence {
    env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.collision_persistence
}
- (())setCollisionPersistence:(u32)p {
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.collision_persistence = p;
}

- (id)staticBody {
    let existing = env.objc.borrow::<ChipmunkHastySpaceHostObject>(this).superclass.static_body;
    if existing != nil { return existing; }
    let body: id = msg_class![env; ChipmunkBody alloc];
    let body: id = msg![env; body init];
    retain(env, body);
    env.objc.borrow_mut::<ChipmunkHastySpaceHostObject>(this).superclass.static_body = body;
    body
}

// MARK: step — identical to ChipmunkSpace (no multithreading)

- (())step:(f32)dt {
    log_dbg!("ChipmunkHastySpace step:{} — stub", dt);
}

// MARK: Description

- (id)description {
    let (gx, gy, threads) = {
        let h = env.objc.borrow::<ChipmunkHastySpaceHostObject>(this);
        (h.superclass.gravity_x, h.superclass.gravity_y, h.threads)
    };
    let s = format!(
        "<ChipmunkHastySpace: {:?}; gravity=({},{}) threads={}>",
        this, gx, gy, threads
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};
