/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSCharacterSet` class cluster, including `NSMutableCharacterSet`.

use touchhle_macros::validate_class_exports;

use super::{ns_string, unichar};
use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, retain, ClassExports, HostObject, NSZonePtr,
};
use std::collections::HashSet;

/// Belongs to _touchHLE_NSCharacterSet
struct CharacterSetHostObject {
    set: HashSet<unichar>,
    inverted: bool,
}
impl HostObject for CharacterSetHostObject {}

#[validate_class_exports]
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSCharacterSet is an abstract class. A subclass must provide:
// - (bool)characterIsMember:(unichar)character
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSCharacterSet.
@implementation NSCharacterSet: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSCharacterSet might be subclassed by something which needs
    // allocWithZone: to have the normal behaviour. Unimplemented: call
    // superclass alloc then.
    assert!(this == env.objc.get_known_class("NSCharacterSet", &mut env.mem));
    msg_class![env; _touchHLE_NSCharacterSet allocWithZone:zone]
}

// This doesn't have a corresponding init method for some reason.
+ (id)characterSetWithCharactersInString:(id)string { // NSString*
    let mut set = HashSet::new();
    ns_string::for_each_code_unit(env, string, |_idx, c| { set.insert(c); });

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

+ (id)whitespaceCharacterSet {
    // Unicode General Category Zs and CHARACTER TABULATION (U+0009).
    let chars = [
        '\u{0020}', '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
        '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}',
        '\u{202F}', '\u{205F}', '\u{3000}', '\u{0009}',
    ];
    let set = HashSet::from(chars.map(|c| unichar::try_from(c).unwrap()));

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    // TODO: override this once we have NSMutableCharacterSet!
    retain(env, this)
}

@end

// Our private subclass that is the single implementation of NSCharacterSet for
// the time being.
@implementation _touchHLE_NSCharacterSet: NSCharacterSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CharacterSetHostObject {
        set: HashSet::new(),
        inverted: false
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: initWithCoder:

- (bool)characterIsMember:(unichar)code_unit {
    let host_object = env.objc.borrow::<CharacterSetHostObject>(this);
    host_object.set.contains(&code_unit) ^ host_object.inverted
}

- (id)invertedSet {
    let old_host_object = env.objc.borrow::<CharacterSetHostObject>(this);
    let host_object = Box::new(CharacterSetHostObject {
        set: old_host_object.set.clone(),
        inverted: !old_host_object.inverted
    });
    let class = env.objc.get_known_class("_touchHLE_NSCharacterSet", &mut env.mem);
    env.objc.alloc_object(class, host_object, &mut env.mem)
}

@end

};
