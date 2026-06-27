/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSSortDescriptor`.

use super::ns_string;
use crate::frameworks::foundation::NSComparisonResult;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};

#[derive(Default)]
struct NSSortDescriptorHostObject {
    /// `NSString*` — key path to compare on
    key: id,
    ascending: bool,
    /// `SEL` — comparison selector (default: `compare:`)
    selector: SEL,
    /// `id<NSComparator>` block — retained, nil if selector-based
    comparator: id,
}
impl HostObject for NSSortDescriptorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSSortDescriptor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let compare_sel = env.objc.register_host_selector("compare:".to_string(), &mut env.mem);
    let host_object = Box::new(NSSortDescriptorHostObject {
        key:        nil,
        ascending:  true,
        selector:   compare_sel,
        comparator: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Constructors

+ (id)sortDescriptorWithKey:(id)key ascending:(bool)ascending { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithKey:key ascending:ascending];
    autorelease(env, new)
}

+ (id)sortDescriptorWithKey:(id)key
                  ascending:(bool)ascending
                   selector:(SEL)selector {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithKey:key ascending:ascending selector:selector];
    autorelease(env, new)
}

+ (id)sortDescriptorWithKey:(id)key
                  ascending:(bool)ascending
                 comparator:(id)comparator { // NSComparator block
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithKey:key ascending:ascending comparator:comparator];
    autorelease(env, new)
}

- (id)initWithKey:(id)key ascending:(bool)ascending { // NSString*
    retain(env, key);
    let host = env.objc.borrow_mut::<NSSortDescriptorHostObject>(this);
    host.key       = key;
    host.ascending = ascending;
    this
}

- (id)initWithKey:(id)key
        ascending:(bool)ascending
          selector:(SEL)selector {
    retain(env, key);
    let host = env.objc.borrow_mut::<NSSortDescriptorHostObject>(this);
    host.key       = key;
    host.ascending = ascending;
    host.selector  = selector;
    this
}

- (id)initWithKey:(id)key
        ascending:(bool)ascending
       comparator:(id)comparator { // NSComparator (id/block)
    retain(env, key);
    retain(env, comparator);
    let host = env.objc.borrow_mut::<NSSortDescriptorHostObject>(this);
    host.key        = key;
    host.ascending  = ascending;
    host.comparator = comparator;
    this
}

- (())dealloc {
    let host = env.objc.borrow::<NSSortDescriptorHostObject>(this);
    let (key, comparator) = (host.key, host.comparator);
    release(env, key);
    release(env, comparator);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Accessors

- (id)key {
    env.objc.borrow::<NSSortDescriptorHostObject>(this).key
}

- (bool)ascending {
    env.objc.borrow::<NSSortDescriptorHostObject>(this).ascending
}

- (SEL)selector {
    env.objc.borrow::<NSSortDescriptorHostObject>(this).selector
}

// MARK: - Comparison

- (NSComparisonResult)compareObject:(id)object1 toObject:(id)object2 {
    let host = env.objc.borrow::<NSSortDescriptorHostObject>(this);
    let key        = host.key;
    let ascending  = host.ascending;
    let selector   = host.selector;
    let comparator = host.comparator;

    // Extract values via valueForKeyPath: if key is set.
    let (val1, val2) = if key != nil {
        let v1: id = msg![env; object1 valueForKeyPath:key];
        let v2: id = msg![env; object2 valueForKeyPath:key];
        (v1, v2)
    } else {
        (object1, object2)
    };

    // Use comparator block if present, otherwise use selector.
    let raw_result: NSComparisonResult = if comparator != nil {
        // NSComparator is `NSComparisonResult (^)(id obj1, id obj2)`.
        // We can't invoke blocks directly; fall back to selector.
        log!("NSSortDescriptor compareObject:toObject: comparator blocks not supported, falling back to selector");
        msg![env; val1 performSelector:selector withObject:val2]
    } else {
        if val1 == nil && val2 == nil { return  0; }
        if val1 == nil               { return  1; } // nil sorts last
        if val2 == nil               { return -1; }
        msg![env; val1 performSelector:selector withObject:val2]
    };

    // Flip if descending.
    if ascending { raw_result } else { -raw_result }
}

// MARK: - Reversed descriptor

- (id)reversedSortDescriptor {
    let host = env.objc.borrow::<NSSortDescriptorHostObject>(this);
    let key       = host.key;
    let ascending = host.ascending;
    let selector  = host.selector;

    let new: id = msg_class![env; NSSortDescriptor alloc];
    let new: id = msg![env; new initWithKey:key ascending:(!ascending) selector:selector];
    autorelease(env, new)
}

// MARK: - NSCopying

- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

// MARK: - NSCoding

- (id)initWithCoder:(id)coder {
    let key_key  = crate::frameworks::foundation::ns_string::get_static_str(env, "NSKey");
    let asc_key  = crate::frameworks::foundation::ns_string::get_static_str(env, "NSAscending");
    let sel_key  = crate::frameworks::foundation::ns_string::get_static_str(env, "NSSelector");

    let key: id       = msg![env; coder decodeObjectForKey:key_key];
    let ascending: bool = msg![env; coder decodeBoolForKey:asc_key];
    let sel_str: id   = msg![env; coder decodeObjectForKey:sel_key];

    retain(env, key);
    let selector = if sel_str != nil {
        let sel_name = ns_string::to_rust_string(env, sel_str).into_owned();
        env.objc.register_host_selector(sel_name, &mut env.mem)
    } else {
        env.objc.register_host_selector("compare:".to_string(), &mut env.mem)
    };

    let host = env.objc.borrow_mut::<NSSortDescriptorHostObject>(this);
    host.key       = key;
    host.ascending = ascending;
    host.selector  = selector;
    this
}

- (())encodeWithCoder:(id)coder {
    let host = env.objc.borrow::<NSSortDescriptorHostObject>(this);
    let (key, ascending, selector) = (host.key, host.ascending, host.selector);

    let key_key = ns_string::get_static_str(env, "NSKey");
    let asc_key = ns_string::get_static_str(env, "NSAscending");
    let sel_key = ns_string::get_static_str(env, "NSSelector");

    () = msg![env; coder encodeObject:key forKey:key_key];
    () = msg![env; coder encodeBool:ascending forKey:asc_key];

    let sel_name = env.objc.get_selector_name(selector).to_string();
    let sel_ns   = ns_string::from_rust_string(env, sel_name);
    () = msg![env; coder encodeObject:sel_ns forKey:sel_key];
    release(env, sel_ns);
}

// MARK: - isEqual / hash / description

- (bool)isEqual:(id)other {
    if this == other { return true; }
    if other == nil  { return false; }

    let a_key: id   = msg![env; this key];
    let b_key: id   = msg![env; other key];
    let a_asc: bool = msg![env; this ascending];
    let b_asc: bool = msg![env; other ascending];

    let keys_eq: bool = if a_key == nil && b_key == nil {
        true
    } else if a_key == nil || b_key == nil {
        false
    } else {
        msg![env; a_key isEqual:b_key]
    };
    keys_eq && a_asc == b_asc
}

- (id)description {
    let host = env.objc.borrow::<NSSortDescriptorHostObject>(this);
    let key  = host.key;
    let asc  = host.ascending;

    let key_str = if key != nil {
        ns_string::to_rust_string(env, key).into_owned()
    } else {
        "<nil>".to_string()
    };

    let s  = format!("(key: {}, ascending: {})", key_str, asc);
    let ns = ns_string::from_rust_string(env, s);
    autorelease(env, ns)
}

@end

};

/// Sort `objects` (an `NSMutableArray*`) using a `descriptors` (`NSArray*`
/// of `NSSortDescriptor*`).  Exposed for use by `NSArray sortedArray…` and
/// `NSMutableArray sortUsingDescriptors:`.
pub fn sort_with_descriptors(
    env: &mut crate::Environment,
    objects: crate::objc::id,
    descriptors: crate::objc::id,
) {
    use crate::objc::msg;

    let count: u32 = msg![env; objects count];
    let desc_count: u32 = msg![env; descriptors count];

    if count == 0 || desc_count == 0 {
        return;
    }

    // Collect into a Rust vec, sort, write back.
    let mut items: Vec<crate::objc::id> = (0..count)
        .map(|i| msg![env; objects objectAtIndex:i])
        .collect();

    // Insertion sort — stable and avoids complex borrow patterns.
    let n = items.len();
    for i in 1..n {
        let mut j = i;
        while j > 0 {
            let a = items[j - 1];
            let b = items[j];
            let mut result: crate::frameworks::foundation::NSComparisonResult = 0;

            for d in 0..desc_count {
                let desc: crate::objc::id = msg![env; descriptors objectAtIndex:d];
                result = msg![env; desc compareObject:a toObject:b];
                if result != 0 {
                    break;
                }
            }
            if result > 0 {
                items.swap(j - 1, j);
                j -= 1;
            } else {
                break;
            }
        }
    }

    for (i, val) in items.iter().enumerate() {
        () = msg![env; objects replaceObjectAtIndex:(i as u32) withObject:(*val)];
    }
}
