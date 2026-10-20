/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSScanner` class.

use crate::frameworks::foundation::ns_string::{from_u16_vec, to_rust_string};
use crate::frameworks::foundation::{unichar, NSNotFound, NSRange, NSUInteger};
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

// TODO: Speed up by optimizing for internal subclasses
#[derive(Default, Clone)]
struct NSScannerHostObject {
    /// NSCharacterSet *, characters to be skipped
    to_be_skipped: id,
    /// NSString *, should always be immutable since it's copied
    string: id,
    /// Length is cached since it is immutable.
    len: NSUInteger,
    pos: NSUInteger,
}
impl HostObject for NSScannerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSScanner: NSObject

+ (id)scannerWithString:(id)string {
    let new: id = msg![env; this alloc];
    let new = msg![env; new initWithString:string];
    autorelease(env, new)
}

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSScanner might be subclassed by something which needs
    // allocWithZone: to have the normal behaviour. Unimplemented: call
    // superclass alloc then.
    assert!(this == env.objc.get_known_class("NSScanner", &mut env.mem));
    msg_class![env; _touchHLE_NSScanner allocWithZone:zone]
}

@end

// Our private subclass that is the single implementation of NSScanner for
// the time being.
@implementation _touchHLE_NSScanner: NSScanner

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSScannerHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithString:(id)string { // NSString *
    assert!(string != nil);
    let string: id = msg![env; string copy]; // Same behaviour as simulator
    let len: NSUInteger = msg![env; string length];
    let default_set = msg_class![env; NSCharacterSet whitespaceAndNewlineCharacterSet];
    retain(env, default_set);
    *env.objc.borrow_mut(this) = NSScannerHostObject {
        to_be_skipped: default_set,
        string,
        len,
        pos: 0
    };
    this
}

- (())dealloc {
    let &NSScannerHostObject {
        to_be_skipped,
        string,
        ..
    } = env.objc.borrow(this);
    release(env, string);
    release(env, to_be_skipped);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (())setCharactersToBeSkipped:(id)to_be_skipped { // NSCharacterSet *
    let old_to_be_skipped = env.objc.borrow::<NSScannerHostObject>(this).to_be_skipped;
    env.objc.borrow_mut::<NSScannerHostObject>(this).to_be_skipped = to_be_skipped;
    retain(env, to_be_skipped);
    release(env, old_to_be_skipped);
}
- (id)charactersToBeSkipped {
    env.objc.borrow::<NSScannerHostObject>(this).to_be_skipped
}

- (NSUInteger)scanLocation {
    env.objc.borrow::<NSScannerHostObject>(this).pos
}

- (())setScanLocation:(NSUInteger)loc {
    let host = env.objc.borrow_mut::<NSScannerHostObject>(this);
    assert!(loc <= host.len); // TODO: raise NSRangeException
    host.pos = loc;
}

- (bool)isAtEnd {
    skip_characters(env, this);
    let NSScannerHostObject { len, pos, .. } = env.objc.borrow::<NSScannerHostObject>(this);
    len == pos
}

- (bool)scanUpToCharactersFromSet:(id)cset intoString:(MutPtr<id>)str {
    skip_characters(env, this);

    let NSScannerHostObject { to_be_skipped, string, len, mut pos } = env.objc.borrow::<NSScannerHostObject>(this).clone();
    if pos >= len {
        // Does nothing (same as simulator)
        return false;
    }
    let first_scan: unichar = msg![env; string characterAtIndex:pos];
    if msg![env; cset characterIsMember:first_scan] {
        // Does nothing (same as simulator)
        return false;
    }
    let mut chars = vec![first_scan];
    pos += 1;
    while pos < len {
        let curr = msg![env; string characterAtIndex:pos];
        if msg![env; cset characterIsMember:curr] {
            break
        }
        pos += 1;
        chars.push(curr);
    }
    if !str.is_null() {
        let out = from_u16_vec(env, chars);
        autorelease(env, out);
        env.mem.write(str, out);
    }

    *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos };
    true
}

- (bool)scanCharactersFromSet:(id)cset intoString:(MutPtr<id>)str {
    let inv_cset: id = msg![env; cset invertedSet];
    msg![env; this scanUpToCharactersFromSet:inv_cset intoString:str]
}

- (bool)scanHexInt:(MutPtr<u32>)result {
    assert!(!result.is_null());
    skip_characters(env, this);

    let NSScannerHostObject { to_be_skipped: _set, string, len, pos } = env.objc.borrow::<NSScannerHostObject>(this).clone();
    assert!(pos < len);
    let susbstring: id = msg![env; string substringFromIndex:pos];
    let tmp = to_rust_string(env, susbstring);
    assert!(!tmp.starts_with("0x") && !tmp.starts_with("0X"));
    assert!(!tmp.chars().next().unwrap().is_ascii_hexdigit()); // TODO
    env.mem.write(result, 0);
    false
}

- (bool)scanUpToString:(id)stop_string // NSString *
            intoString:(MutPtr<id>)result { // NSString **
    skip_characters(env, this);

    let NSScannerHostObject { to_be_skipped, string, len, pos } = std::mem::take(env.objc.borrow_mut::<NSScannerHostObject>(this));
    log_dbg!("scanUpToString:'{}' intoString: from '{}' at {}", to_rust_string(env, stop_string), to_rust_string(env, string), pos);

    // TODO: avoid string copying
    let left: id = msg![env; string substringFromIndex:pos];
    let range: NSRange = msg![env; left rangeOfString:stop_string];
    if range.location == 0 {
        *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos };
        return false;
    }

    let scan_len = if range.location == NSNotFound as NSUInteger {
        len - pos
    } else {
        range.location
    };
    assert!(pos + scan_len <= len);
    *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos: pos + scan_len };

    if !result.is_null() {
        let copy: id = msg![env; left substringToIndex:scan_len];
        log_dbg!("scanned '{}' up to {}", to_rust_string(env, copy), pos + scan_len);
        // Note: substring is already autoreleased
        env.mem.write(result, copy);
    }
    true
}

- (bool)scanString:(id)scan_string // NSString *
        intoString:(MutPtr<id>)result { // NSString **
    skip_characters(env, this);

    let NSScannerHostObject { to_be_skipped, string, len, pos } = std::mem::take(env.objc.borrow_mut::<NSScannerHostObject>(this));
    log_dbg!("scanString:{} intoString: from '{}' at {}", to_rust_string(env, scan_string), to_rust_string(env, string), pos);

    // TODO: avoid string copying
    let left: id = msg![env; string substringFromIndex:pos];
    let same_prefix: bool = msg![env; left hasPrefix:scan_string];
    if !same_prefix {
        *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos };
        return false;
    }

    let scan_len: NSUInteger = msg![env; scan_string length];
    assert!(pos + scan_len <= len);
    *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos: pos + scan_len };

    if !result.is_null() {
        let copy: id = msg![env; scan_string copy];
        autorelease(env, copy);
        env.mem.write(result, copy);
    }
    true
}

- (bool)scanInt:(MutPtr<i32>)result {
    skip_characters(env, this);

    let NSScannerHostObject { to_be_skipped, string, len, pos } = std::mem::take(env.objc.borrow_mut::<NSScannerHostObject>(this));
    let left: id = msg![env; string substringFromIndex:pos];
    if left == nil {
        *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos };
        return false;
    }

    let st = to_rust_string(env, left);
    let mut cutoff = st.len();
    for (i, c) in st.char_indices() {
        if !c.is_ascii_digit() && c != '+' && c != '-' {
            cutoff = i;
            break;
        }
    }
    if cutoff == 0 {
        log_dbg!("scanInt: no valid int found for '{}'", st);
        *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos };
        return false;
    }

    if !result.is_null() {
        // TODO: handle over/underflow properly
        let res = st[..cutoff].parse().unwrap_or(0);
        log_dbg!("scanInt: from '{}' -> {}", st, res);
        env.mem.write(result, res);
    }

    *env.objc.borrow_mut::<NSScannerHostObject>(this) = NSScannerHostObject { to_be_skipped, string, len, pos: pos + cutoff as NSUInteger };
    true
}

@end

};

// Helper functions, skips characters from `charactersToBeSkipped` set
fn skip_characters(env: &mut Environment, scanner: id) {
    let &NSScannerHostObject {
        to_be_skipped,
        string,
        len,
        mut pos,
    } = env.objc.borrow::<NSScannerHostObject>(scanner);
    loop {
        if pos >= len {
            break;
        }
        let c: unichar = msg![env; string characterAtIndex:pos];
        if msg![env; to_be_skipped characterIsMember:c] {
            pos += 1;
        } else {
            break;
        }
    }
    env.objc.borrow_mut::<NSScannerHostObject>(scanner).pos = pos;
}
