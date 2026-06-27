/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSCharacterSet` class cluster, including `NSMutableCharacterSet`.

use super::{ns_string, unichar, NSRange, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, retain, ClassExports, HostObject, NSZonePtr,
};
use std::collections::HashSet;

// Unicode General Category Zs and CHARACTER TABULATION (U+0009).
const WHITESPACE_CHARACTERS: [char; 18] = [
    '\u{0020}', '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}',
    '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}',
    '\u{3000}', '\u{0009}',
];
// The newline characters (U+000A - U+000D, U+0085, U+2028, and U+2029).
const NEWLINE_CHARACTERS: [char; 7] = [
    '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0085}', '\u{2028}', '\u{2029}',
];

// Unicode General Category Cc and Cf.
// TODO: This is not actually the full list of charadcters in Cc and Cf, but
// these are the ones reported from a simulator run with all `unichar`s. This
// excludes characters outside the BMP, whcih would only be needed when we
// support -longCharacterIsMember:.
const CONTROL_CHARACTERS: [char; 99] = [
    '\u{0000}', '\u{0001}', '\u{0002}', '\u{0003}', '\u{0004}', '\u{0005}', '\u{0006}', '\u{0007}',
    '\u{0008}', '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{000E}', '\u{000F}',
    '\u{0010}', '\u{0011}', '\u{0012}', '\u{0013}', '\u{0014}', '\u{0015}', '\u{0016}', '\u{0017}',
    '\u{0018}', '\u{0019}', '\u{001A}', '\u{001B}', '\u{001C}', '\u{001D}', '\u{001E}', '\u{001F}',
    '\u{007F}', '\u{0080}', '\u{0081}', '\u{0082}', '\u{0083}', '\u{0084}', '\u{0085}', '\u{0086}',
    '\u{0087}', '\u{0088}', '\u{0089}', '\u{008A}', '\u{008B}', '\u{008C}', '\u{008D}', '\u{008E}',
    '\u{008F}', '\u{0090}', '\u{0091}', '\u{0092}', '\u{0093}', '\u{0094}', '\u{0095}', '\u{0096}',
    '\u{0097}', '\u{0098}', '\u{0099}', '\u{009A}', '\u{009B}', '\u{009C}', '\u{009D}', '\u{009E}',
    '\u{009F}', '\u{00AD}', '\u{0600}', '\u{0601}', '\u{0602}', '\u{0603}', '\u{06DD}', '\u{070F}',
    '\u{17B4}', '\u{17B5}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{200E}', '\u{200F}', '\u{202A}',
    '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2060}', '\u{2061}', '\u{2062}', '\u{2063}',
    '\u{2064}', '\u{206A}', '\u{206B}', '\u{206C}', '\u{206D}', '\u{206E}', '\u{206F}', '\u{FEFF}',
    '\u{FFF9}', '\u{FFFA}', '\u{FFFB}',
];

/// Belongs to _touchHLE_NSCharacterSet
struct CharacterSetHostObject {
    set: HashSet<unichar>,
    inverted: bool,
}
impl HostObject for CharacterSetHostObject {}

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

+ (id)characterSetWithRange:(NSRange)range {
    let mut set = HashSet::new();

    for c in range.location..range.location.checked_add(range.length).unwrap() {
        set.insert(unichar::try_from(c).unwrap());
    }

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

+ (id)newlineCharacterSet {
    let set = HashSet::from(NEWLINE_CHARACTERS.map(|c| unichar::try_from(c).unwrap()));

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

+ (id)whitespaceCharacterSet {
    let set = HashSet::from(WHITESPACE_CHARACTERS.map(|c| unichar::try_from(c).unwrap()));

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

+ (id)whitespaceAndNewlineCharacterSet {
    let set1 = HashSet::from(NEWLINE_CHARACTERS.map(|c| unichar::try_from(c).unwrap()));
    let set2 = HashSet::from(WHITESPACE_CHARACTERS.map(|c| unichar::try_from(c).unwrap()));
    let set = set1.union(&set2).copied().collect();

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

+ (id)controlCharacterSet {
    let set = HashSet::from(CONTROL_CHARACTERS.map(|c| unichar::try_from(c).unwrap()));

    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<CharacterSetHostObject>(new).set = set;

    autorelease(env, new)
}

@end

// NSMutableCharacterSet defines no primitive methods. Subclasses must
// implement all methods declared by this class in addition to the
// primitives of NSCharacterSet. They must also implement mutableCopyWithZone:.
@implementation NSMutableCharacterSet: NSCharacterSet

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSMutableCharacterSet might be subclassed by something which needs
    // allocWithZone: to have the normal behaviour. Unimplemented: call
    // superclass alloc then.
    assert!(this == env.objc.get_known_class("NSMutableCharacterSet", &mut env.mem));
    msg_class![env; _touchHLE_NSMutableCharacterSet allocWithZone:zone]
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

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}
// NSMutableCopying implementation
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    todo!()
}

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

// Our private subclass that is the single implementation of
// NSMutableCharacterSet for the time being.
@implementation _touchHLE_NSMutableCharacterSet: NSMutableCharacterSet

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CharacterSetHostObject {
        set: HashSet::new(),
        inverted: false
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    todo!()
}
// NSMutableCopying implementation
- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    todo!()
}

- (bool)characterIsMember:(unichar)code_unit {
    let host_object = env.objc.borrow::<CharacterSetHostObject>(this);
    host_object.set.contains(&code_unit) ^ host_object.inverted
}

- (())addCharactersInString:(id)string { // NSString *
    assert!(!env.objc.borrow::<CharacterSetHostObject>(this).inverted); // TODO
    let length: NSUInteger = msg![env; string length];
    for i in 0..length {
        let c = msg![env; string characterAtIndex:i];
        env.objc.borrow_mut::<CharacterSetHostObject>(this).set.insert(c);
    }
}

@end

};
