/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSPredicate`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// =========================================================================
// MARK: - Predicate kind
// =========================================================================

enum PredicateKind {
    /// Always evaluates to the given constant. Also acts as the
    /// `Default` for the phantom-fallback host object (yielding a
    /// predicate that always returns `false`).
    Constant(bool),
    /// Raw format string — stored but not parsed/evaluated.
    Format(String),
    /// NOT of another predicate.
    Not(id),
    /// AND of two predicates.
    And(id, id),
    /// OR of two predicates.
    Or(id, id),
    /// Block-based predicate (not supported — always returns false).
    Block,
}
impl Default for PredicateKind {
    fn default() -> Self {
        PredicateKind::Constant(false)
    }
}

#[derive(Default)]
struct NSPredicateHostObject {
    kind: PredicateKind,
    /// Original format string if created with predicateWithFormat:.
    format: Option<String>,
}
impl HostObject for NSPredicateHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSPredicate: NSObject

// =========================================================================
// MARK: - Allocation
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSPredicateHostObject {
        kind: PredicateKind::Constant(false),
        format: None,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// =========================================================================
// MARK: - Convenience constructors
// =========================================================================

// predicateWithValue: — always-true or always-false predicate.
+ (id)predicateWithValue:(bool)value {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithValue:value];
    crate::objc::autorelease(env, new)
}

// predicateWithFormat: — stores the format string; evaluation is stubbed.
+ (id)predicateWithFormat:(id)format { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithFormat:format];
    crate::objc::autorelease(env, new)
}

// predicateWithFormat:argumentArray:
+ (id)predicateWithFormat:(id)format
           argumentArray:(id)_arguments { // NSArray*
    // Argument substitution is not implemented — treat as plain format.
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithFormat:format];
    crate::objc::autorelease(env, new)
}

// predicateWithBlock: — block evaluation is not supported; the predicate
// always evaluates to false (and logs when evaluated).
+ (id)predicateWithBlock:(id)_block {
    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSPredicateHostObject>(new).kind = PredicateKind::Block;
    crate::objc::autorelease(env, new)
}

// notPredicateWithSubpredicate:
+ (id)notPredicateWithSubpredicate:(id)predicate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithNotPredicate:predicate];
    crate::objc::autorelease(env, new)
}

// andPredicateWithSubpredicates:
+ (id)andPredicateWithSubpredicates:(id)subpredicates { // NSArray*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithAndPredicates:subpredicates];
    crate::objc::autorelease(env, new)
}

// orPredicateWithSubpredicates:
+ (id)orPredicateWithSubpredicates:(id)subpredicates { // NSArray*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithOrPredicates:subpredicates];
    crate::objc::autorelease(env, new)
}

// =========================================================================
// MARK: - Initializers
// =========================================================================

- (id)initWithValue:(bool)value {
    env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
        PredicateKind::Constant(value);
    this
}

- (id)initWithFormat:(id)format { // NSString*
    let fmt_str = if format != nil {
        ns_string::to_rust_string(env, format).into_owned()
    } else {
        String::new()
    };
    log_dbg!("NSPredicate initWithFormat: {:?}", fmt_str);
    {
        let host = env.objc.borrow_mut::<NSPredicateHostObject>(this);
        host.format = Some(fmt_str.clone());
        host.kind = PredicateKind::Format(fmt_str);
    }
    this
}

- (id)initWithNotPredicate:(id)subpredicate {
    retain(env, subpredicate);
    env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
        PredicateKind::Not(subpredicate);
    this
}

- (id)initWithAndPredicates:(id)subpredicates { // NSArray*
    // For simplicity store as AND of first two; a full implementation would
    // fold the entire array. Most callers pass exactly two predicates.
    let count: crate::frameworks::foundation::NSUInteger =
        msg![env; subpredicates count];
    if count == 0 {
        env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
            PredicateKind::Constant(true); // vacuously true
    } else if count == 1 {
        // A single-element AND evaluates the same as its only subpredicate.
        // Store it as And(p, p): p && p == p, and dealloc releases both
        // retains.
        let p: id = msg![env; subpredicates objectAtIndex:0u32];
        retain(env, p);
        retain(env, p);
        env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
            PredicateKind::And(p, p);
    } else {
        let p0: id = msg![env; subpredicates objectAtIndex:0u32];
        let p1: id = msg![env; subpredicates objectAtIndex:1u32];
        retain(env, p0);
        retain(env, p1);
        env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
            PredicateKind::And(p0, p1);
    }
    this
}

- (id)initWithOrPredicates:(id)subpredicates { // NSArray*
    let count: crate::frameworks::foundation::NSUInteger =
        msg![env; subpredicates count];
    if count == 0 {
        env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
            PredicateKind::Constant(false); // vacuously false
    } else if count == 1 {
        // A single-element OR evaluates the same as its only subpredicate.
        let p: id = msg![env; subpredicates objectAtIndex:0u32];
        retain(env, p);
        retain(env, p);
        env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
            PredicateKind::Or(p, p);
    } else {
        let p0: id = msg![env; subpredicates objectAtIndex:0u32];
        let p1: id = msg![env; subpredicates objectAtIndex:1u32];
        retain(env, p0);
        retain(env, p1);
        env.objc.borrow_mut::<NSPredicateHostObject>(this).kind =
            PredicateKind::Or(p0, p1);
    }
    this
}

// =========================================================================
// MARK: - Dealloc
// =========================================================================

- (())dealloc {
    let kind = std::mem::replace(
        &mut env.objc.borrow_mut::<NSPredicateHostObject>(this).kind,
        PredicateKind::Constant(false),
    );
    match kind {
        PredicateKind::Not(p) => release(env, p),
        PredicateKind::And(a, b) | PredicateKind::Or(a, b) => {
            release(env, a);
            release(env, b);
        }
        _ => {}
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// MARK: - NSCopying
// =========================================================================

- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

// =========================================================================
// MARK: - Evaluation
// =========================================================================

// evaluateWithObject: — evaluate against a single object.
- (bool)evaluateWithObject:(id)object {
    evaluate_predicate(env, this, object)
}

// evaluateWithObject:substitutionVariables: — variable substitution not
// implemented; delegates to plain evaluateWithObject:.
- (bool)evaluateWithObject:(id)object
   substitutionVariables:(id)_variables { // NSDictionary*
    evaluate_predicate(env, this, object)
}

// =========================================================================
// MARK: - Predicate string / subpredicates
// =========================================================================

- (id)predicateFormat { // NSString*
    let fmt = {
        let host = env.objc.borrow::<NSPredicateHostObject>(this);
        match &host.kind {
            PredicateKind::Constant(v) => {
                if *v { "TRUEPREDICATE".to_string() } else { "FALSEPREDICATE".to_string() }
            }
            PredicateKind::Format(s) => s.clone(),
            PredicateKind::Not(_)    => "NOT (...)".to_string(),
            PredicateKind::And(_, _) => "(...) AND (...)".to_string(),
            PredicateKind::Or(_, _)  => "(...) OR (...)".to_string(),
            PredicateKind::Block     => "BLOCK".to_string(),
        }
    };
    let s: id = ns_string::from_rust_string(env, fmt);
    crate::objc::autorelease(env, s)
}

// predicateWithSubstitutionVariables: — returns self (no substitution).
- (id)predicateWithSubstitutionVariables:(id)_variables {
    retain(env, this);
    crate::objc::autorelease(env, this)
}

// =========================================================================
// MARK: - Compound predicate subpredicates accessors
// =========================================================================

- (id)subpredicates { // NSArray*
    let (a, b) = {
        let host = env.objc.borrow::<NSPredicateHostObject>(this);
        match &host.kind {
            PredicateKind::And(a, b) | PredicateKind::Or(a, b) => (*a, *b),
            PredicateKind::Not(p) => (*p, nil),
            _ => (nil, nil),
        }
    };
    if a == nil {
        return msg_class![env; NSArray array];
    }
    let arr: id = msg_class![env; NSMutableArray new];
    let _: () = msg![env; arr addObject:a];
    if b != nil {
        let _: () = msg![env; arr addObject:b];
    }
    crate::objc::autorelease(env, arr)
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let fmt: id = msg![env; this predicateFormat];
    let fmt_str = ns_string::to_rust_string(env, fmt);
    let s = format!("<NSPredicate: {:?} \"{}\">", this, fmt_str);
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - NSCompoundPredicate (subclass — thin wrapper)
// =========================================================================

@implementation NSCompoundPredicate: NSPredicate

+ (id)notPredicateWithSubpredicate:(id)predicate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithNotPredicate:predicate];
    crate::objc::autorelease(env, new)
}

+ (id)andPredicateWithSubpredicates:(id)subpredicates {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithAndPredicates:subpredicates];
    crate::objc::autorelease(env, new)
}

+ (id)orPredicateWithSubpredicates:(id)subpredicates {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithOrPredicates:subpredicates];
    crate::objc::autorelease(env, new)
}

@end

// =========================================================================
// MARK: - NSComparisonPredicate (subclass — stub)
// =========================================================================

@implementation NSComparisonPredicate: NSPredicate

+ (id)predicateWithLeftExpression:(id)_lhs
                  rightExpression:(id)_rhs
                         modifier:(i32)_modifier
                             type:(i32)_type
                          options:(i32)_options {
    log!("TODO: NSComparisonPredicate predicateWithLeftExpression: — returning false predicate");
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithValue:false];
    crate::objc::autorelease(env, new)
}

+ (id)predicateWithLeftExpression:(id)_lhs
                  rightExpression:(id)_rhs
               customSelector:(id)_selector {
    log!("TODO: NSComparisonPredicate predicateWithLeftExpression:customSelector: — returning false predicate");
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithValue:false];
    crate::objc::autorelease(env, new)
}

@end

};

// =========================================================================
// MARK: - Evaluation helper
// =========================================================================

fn evaluate_predicate(
    env: &mut crate::Environment,
    predicate: crate::objc::id,
    object: crate::objc::id,
) -> bool {
    // Read kind without holding borrow across recursive calls.
    let kind_snapshot = {
        let host = env.objc.borrow::<NSPredicateHostObject>(predicate);
        match &host.kind {
            PredicateKind::Constant(v) => return *v,
            PredicateKind::Block => {
                log!("NSPredicate block evaluation not supported — returning false");
                return false;
            }
            PredicateKind::Format(s) => {
                log_dbg!(
                    "NSPredicate evaluateWithObject: format predicate {:?} — returning false (not parsed)",
                    s
                );
                return false;
            }
            PredicateKind::Not(p) => ('n', *p, nil),
            PredicateKind::And(a, b) => ('a', *a, *b),
            PredicateKind::Or(a, b) => ('o', *a, *b),
        }
    };
    match kind_snapshot {
        ('n', p, _) => !evaluate_predicate(env, p, object),
        ('a', a, b) => evaluate_predicate(env, a, object) && evaluate_predicate(env, b, object),
        ('o', a, b) => evaluate_predicate(env, a, object) || evaluate_predicate(env, b, object),
        _ => false,
    }
}
