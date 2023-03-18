/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The `NSString` class cluster, including `NSMutableString`.

use super::ns_array;
use super::NSUInteger;
use crate::frameworks::core_graphics::{CGRect, CGSize};
use crate::frameworks::uikit::ui_font::{
    self, UILineBreakMode, UILineBreakModeWordWrap, UITextAlignment, UITextAlignmentLeft,
};
use crate::fs::GuestPath;
use crate::mach_o::MachO;
use crate::mem::{guest_size_of, ConstPtr, Mem, MutPtr, Ptr, SafeRead};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, retain, Class, ClassExports, HostObject,
    NSZonePtr, ObjC,
};
use crate::Environment;
use std::borrow::Cow;
use std::collections::HashMap;
use std::string::FromUtf16Error;

pub type NSStringEncoding = NSUInteger;
pub const NSASCIIStringEncoding: NSUInteger = 1;
pub const NSUTF8StringEncoding: NSUInteger = 4;
pub const NSUnicodeStringEncoding: NSUInteger = 10;
pub const NSUTF16StringEncoding: NSUInteger = NSUnicodeStringEncoding;

pub const NSMaximumStringLength: NSUInteger = (i32::MAX - 1) as _;

#[derive(Default)]
pub struct State {
    static_str_pool: HashMap<&'static str, id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_string
    }
}

/// Constant strings embedded in the app binary use this struct. The name is
/// according to Ghidra, the rest is guesswork.
#[allow(non_camel_case_types)]
struct cfstringStruct {
    _isa: Class,
    flags: u32,
    bytes: ConstPtr<u8>,
    length: NSUInteger,
}
unsafe impl SafeRead for cfstringStruct {}

type Utf16String = Vec<u16>;

/// Belongs to _touchHLE_NSString.
enum StringHostObject {
    Utf8(Cow<'static, str>),
    /// Not necessarily well-formed UTF-16: might contain unpaired surrogates.
    Utf16(Utf16String),
}
impl HostObject for StringHostObject {}
impl StringHostObject {
    fn decode(bytes: Cow<[u8]>, encoding: NSStringEncoding) -> StringHostObject {
        if bytes.len() == 0 {
            return StringHostObject::Utf8(Cow::Borrowed(""));
        }

        // TODO: error handling

        match encoding {
            NSUTF8StringEncoding => {
                let string = String::from_utf8(bytes.into_owned()).unwrap();
                StringHostObject::Utf8(Cow::Owned(string))
            }
            NSUTF16StringEncoding => {
                assert!(bytes.len() % 2 == 0);

                let is_big_endian = match &bytes[0..2] {
                    [0xFE, 0xFF] => true,
                    [0xFF, 0xFE] => false,
                    // TODO: what does NSUnicodeStringEncoding mean if no BOM is
                    // present?
                    _ => unimplemented!("Default endianness"),
                };
                // TODO: Should the BOM be stripped? Always/sometimes/never?

                StringHostObject::Utf16(if is_big_endian {
                    bytes
                        .chunks(2)
                        .map(|chunk| u16::from_be_bytes(chunk.try_into().unwrap()))
                        .collect()
                } else {
                    bytes
                        .chunks(2)
                        .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                        .collect()
                })
            }
            _ => panic!("Unimplemented encoding: {}", encoding),
        }
    }
    fn to_utf8(&self) -> Result<Cow<'static, str>, FromUtf16Error> {
        match self {
            StringHostObject::Utf8(utf8) => Ok(utf8.clone()),
            StringHostObject::Utf16(utf16) => Ok(Cow::Owned(String::from_utf16(utf16)?)),
        }
    }
    /// Mutate the object, converting to UTF-16 if the string was not already
    /// UTF-16. Returns a reference to the UTF-16 content and a boolean that is
    /// [true] if a conversion happened.
    fn convert_to_utf16_inplace(&mut self) -> (&mut Utf16String, bool) {
        let converted = match self {
            Self::Utf8(_) => {
                *self = Self::Utf16(self.iter_code_units().collect());
                true
            }
            Self::Utf16(_) => false,
        };
        let Self::Utf16(utf16) = self else {
            unreachable!();
        };
        (utf16, converted)
    }
    /// Iterate over the string as UTF-16 code units.
    fn iter_code_units(&self) -> CodeUnitIterator {
        match self {
            StringHostObject::Utf8(utf8) => CodeUnitIterator::Utf8(utf8.encode_utf16()),
            StringHostObject::Utf16(utf16) => CodeUnitIterator::Utf16(utf16.iter()),
        }
    }
}

enum CodeUnitIterator<'a> {
    Utf8(std::str::EncodeUtf16<'a>),
    Utf16(std::slice::Iter<'a, u16>),
}
impl<'a> Iterator for CodeUnitIterator<'a> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        match self {
            CodeUnitIterator::Utf8(iter) => iter.next(),
            CodeUnitIterator::Utf16(iter) => iter.next().copied(),
        }
    }
}
impl<'a> Clone for CodeUnitIterator<'a> {
    fn clone(&self) -> Self {
        match self {
            CodeUnitIterator::Utf8(iter) => CodeUnitIterator::Utf8(iter.clone()),
            CodeUnitIterator::Utf16(iter) => CodeUnitIterator::Utf16(iter.clone()),
        }
    }
}
impl<'a> CodeUnitIterator<'a> {
    /// If the sequence of code units in `prefix` is a prefix of `self`,
    /// return [Some] with `self` advanced past that prefix, otherwise [None].
    fn strip_prefix(&self, prefix: &CodeUnitIterator) -> Option<Self> {
        let mut self_match = self.clone();
        let mut prefix_match = prefix.clone();
        loop {
            match prefix_match.next() {
                None => {
                    return Some(self_match);
                }
                Some(prefix_c) => {
                    let self_c = self_match.next();
                    if self_c != Some(prefix_c) {
                        return None;
                    }
                }
            }
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSString is an abstract class. A subclass must provide:
// - (NSUInteger)length;
// - (unichar)characterAtIndex:(NSUInteger)index;
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSString.
@implementation NSString: NSObject

+ (id)allocWithZone:(NSZonePtr)zone {
    // NSString might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSString", &mut env.mem));
    msg_class![env; _touchHLE_NSString allocWithZone:zone]
}

+ (id)stringWithCString:(ConstPtr<u8>)c_string {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCString:c_string];
    autorelease(env, new)
}

+ (id)stringWithCString:(ConstPtr<u8>)c_string
               encoding:(NSStringEncoding)encoding {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCString:c_string encoding:encoding];
    autorelease(env, new)
}

+ (id)stringWithContentsOfFile:(id)path // NSString*
                      encoding:(NSStringEncoding)encoding
                         error:(MutPtr<id>)error { // NSError**
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfFile:path
                                              encoding:encoding
                                                 error:error];
    autorelease(env, new)
}

+ (id)stringWithUTF8String:(ConstPtr<u8>)c_string {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithUTF8String:c_string];
    log!("[NSString stringWithUTF8String]: {:?} -> {:?}", env.mem.cstr_at_utf8(c_string).unwrap(), new);
    autorelease(env, new)
}

// These are the two methods that have to be overridden by subclasses, so these
// implementations don't have to care about foreign subclasses.
- (NSUInteger)length {
    let host_object = env.objc.borrow_mut::<StringHostObject>(this);

    // To know what length the string has in UTF-16, we need to convert it to
    // UTF-16. If `length` is used, it's likely other methods that operate on
    // UTF-16 code unit boundaries will also be used (e.g. `characterAt:`), so
    // persisting the UTF-16 version lets us potentially optimize future method
    // calls. This is a heuristic though and won't always be optimal.
    let (utf16, did_convert) = host_object.convert_to_utf16_inplace();
    if did_convert {
        log_dbg!("[{:?} length]: converted string to UTF-16", this);
    }

    utf16.len().try_into().unwrap()
}
- (u16)characterAtIndex:(NSUInteger)index {
    let host_object = env.objc.borrow_mut::<StringHostObject>(this);

    // The string has to be in UTF-16 to get O(1) rather than O(n) indexing, and
    // it's likely this method will be called many times, so converting it to
    // UTF-16 as early as possible and persisting that representation is
    // probably best for performance. This is a heuristic though and won't
    // always be optimal.
    let (utf16, did_convert) = host_object.convert_to_utf16_inplace();
    if did_convert {
        log_dbg!("[{:?} characterAtIndex:{:?}]: converted string to UTF-16", this, index);
    }

    // TODO: raise exception instead of panicking?
    utf16[index as usize]
}

- (NSUInteger)hash {
    // TODO: avoid copying
    super::hash_helper(&to_rust_string(env, this))
}
- (bool)isEqualTo:(id)other {
    if this == other {
        return true;
    }
    let class: Class = msg_class![env; NSString class];
    if !msg![env; other isKindOfClass:class] {
        return false;
    }
    // TODO: avoid copying
    to_rust_string(env, this) == to_rust_string(env, other)
}
- (bool)isEqualToString:(id)other { // NSString*
    if this == other {
        return true;
    }
    // TODO: avoid copying
    to_rust_string(env, this) == to_rust_string(env, other)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    // TODO: override this once we have NSMutableString!
    retain(env, this)
}

- (bool)getCString:(MutPtr<u8>)buffer
         maxLength:(NSUInteger)buffer_size
          encoding:(NSStringEncoding)encoding {
    assert!(encoding == NSUTF8StringEncoding || encoding == NSASCIIStringEncoding); // TODO: other encodings

    let src = to_rust_string(env, this);
    let dest = env.mem.bytes_at_mut(buffer, buffer_size);
    if dest.len() < src.as_bytes().len() + 1 { // include null terminator
        return false;
    }

    for (i, &byte) in src.as_bytes().iter().chain(b"\0".iter()).enumerate() {
        dest[i] = byte;
    }

    true
}
- (())getCString:(MutPtr<u8>)buffer {
    // This is a deprecated method nobody should use, but unfortunately, it is
    // used. The encoding it should use is [NSString defaultCStringEncoding]
    // but I don't want to figure out what that is on all platforms, and the use
    // I've seen of this method was on ASCII strings, so let's just hardcode
    // UTF-8 and hope that works.

    // Prevent slice out-of-range error
    let length = (u32::MAX - buffer.to_bits()).min(NSMaximumStringLength);
    let res: bool = msg![env; this getCString:buffer
                                    maxLength:length
                                     encoding:NSUTF8StringEncoding];
    assert!(res);
}

- (id)componentsSeparatedByString:(id)separator { // NSString*
    // TODO: support foreign subclasses (perhaps via a helper function that
    // copies the string first)
    let mut main_iter = env.objc.borrow::<StringHostObject>(this)
        .iter_code_units();
    let sep_iter = env.objc.borrow::<StringHostObject>(separator)
        .iter_code_units();

    // TODO: zero-length separator support
    assert!(sep_iter.clone().next().is_some());

    let mut components = Vec::<Utf16String>::new();
    let mut current_component: Utf16String = Vec::new();
    loop {
        if let Some(new_main_iter) = main_iter.strip_prefix(&sep_iter) {
            // matched separator, end current component
            components.push(std::mem::take(&mut current_component));
            main_iter = new_main_iter;
        } else {
            // no separator match, extend the current component
            match main_iter.next() {
                Some(cur) => current_component.push(cur),
                None => break,
            }
        }
    }
    components.push(current_component);

    // TODO: For a foreign subclass of NSString, do we have to return that
    // subclass? The signature implies this isn't the case and it's probably not
    // worth the effort, but it's an interesting question.
    let class = env.objc.get_known_class("_touchHLE_NSString", &mut env.mem);

    let component_ns_strings = components.drain(..).map(|utf16| {
        let host_object = Box::new(StringHostObject::Utf16(utf16));
        env.objc.alloc_object(class, host_object, &mut env.mem)
    }).collect();
    ns_array::from_vec(env, component_ns_strings)
}

- (id)stringByTrimmingCharactersInSet:(id)set { // NSCharacterSet*
    let initial_length: NSUInteger = msg![env; this length];

    let mut res_start: NSUInteger = 0;
    let mut res_end = initial_length;

    while res_start < initial_length {
        let c: u16 = msg![env; this characterAtIndex:res_start];
        if msg![env; set characterIsMember:c] {
            res_start += 1;
        } else {
            break;
        }
    }

    while res_end > res_start {
        let c: u16 = msg![env; this characterAtIndex:(res_end - 1)];
        if msg![env; set characterIsMember:c] {
            res_end -= 1;
        } else {
            break;
        }
    }

    assert!(res_end >= res_start);
    let res_length = res_end - res_start;

    let res = if res_length == initial_length {
        retain(env, this)
    } else {
        // TODO: just call `substringWithRange:` here instead, the only reason
        // the current code doesn't is that it would require figuring out the
        // ABI of NSRange.
        let mut res_utf16: Utf16String = Vec::with_capacity(res_length as usize);

        for_each_code_unit(env, this, |idx, c| {
            if res_start <= idx && idx < res_end {
                res_utf16.push(c);
            }
        });

        let res = msg_class![env; _touchHLE_NSString alloc];
        *env.objc.borrow_mut(res) = StringHostObject::Utf16(res_utf16);
        res
    };
    autorelease(env, res)
}

- (id)stringByReplacingOccurrencesOfString:(id)target // NSString*
                                withString:(id)replacement { // NSString*
    // TODO: support foreign subclasses (perhaps via a helper function that
    // copies the string first)
    let mut main_iter = env.objc.borrow::<StringHostObject>(this)
        .iter_code_units();
    let target_iter = env.objc.borrow::<StringHostObject>(target)
        .iter_code_units();
    let replacement_iter = env.objc.borrow::<StringHostObject>(replacement)
        .iter_code_units();

    // TODO: zero-length target support?
    assert!(target_iter.clone().next().is_some());

    let mut result: Utf16String = Vec::new();
    loop {
        if let Some(new_main_iter) = main_iter.strip_prefix(&target_iter) {
            // matched target, replace it
            result.extend(replacement_iter.clone());
            main_iter = new_main_iter;
        } else {
            // no match, copy as normal
            match main_iter.next() {
                Some(cur) => result.push(cur),
                None => break,
            }
        }
    }

    // TODO: For a foreign subclass of NSString, do we have to return that
    // subclass? The signature implies this isn't the case and it's probably not
    // worth the effort, but it's an interesting question.
    let result_ns_string = msg_class![env; _touchHLE_NSString alloc];
    *env.objc.borrow_mut(result_ns_string) = StringHostObject::Utf16(result);
    autorelease(env, result_ns_string)
}

- (id)stringByAppendingString:(id)other { // NSString*
    assert!(other != nil); // TODO: raise exception

    // TODO: ideally, don't convert to UTF-16 here
    let this_len: NSUInteger = msg![env; this length];
    let other_len: NSUInteger = msg![env; other length];
    let mut new_utf16 = Vec::with_capacity((this_len + other_len) as usize);
    for_each_code_unit(env, this, |_idx, c| {
        new_utf16.push(c);
    });
    for_each_code_unit(env, other, |_idx, c| {
        new_utf16.push(c);
    });

    // TODO: For a foreign subclass of NSString, do we have to return that
    // subclass? The signature implies this isn't the case and it's probably not
    // worth the effort, but it's an interesting question.
    let class = env.objc.get_known_class("_touchHLE_NSString", &mut env.mem);
    let host_object = Box::new(StringHostObject::Utf16(new_utf16));
    env.objc.alloc_object(class, host_object, &mut env.mem)
}

- (id)stringByDeletingLastPathComponent {
    let string = to_rust_string(env, this); // TODO: avoid copying
    let path = GuestPath::new(&string);
    let parent = path.parent().unwrap_or(path);
    let new_string = from_rust_string(env, String::from(parent.as_str()));
    autorelease(env, new_string);
    new_string
}

- (id)stringByAppendingPathComponent:(id)component { // NSString*
    // TODO: avoid copying
    // FIXME: check if Rust join() matches NSString (it probably doesn't)
    let combined = GuestPath::new(&to_rust_string(env, this))
        .join(to_rust_string(env, component));
    let new_string = from_rust_string(env, String::from(combined));
    autorelease(env, new_string);
    new_string
}

- (id)stringByAppendingPathExtension:(id)extension { // NSString*
    // FIXME: handle edge cases like trailing '/' (may differ from Rust!)
    let mut combined = to_rust_string(env, this).into_owned();
    combined.push('.');
    // TODO: avoid copying
    combined.push_str(&to_rust_string(env, extension));

    let new_string = from_rust_string(env, combined);
    autorelease(env, new_string)
}

- (ConstPtr<u8>)UTF8String {
    // TODO: avoid copying
    let string = to_rust_string(env, this);
    let c_string = env.mem.alloc_and_write_cstr(string.as_bytes()).cast_const();
    let length: NSUInteger = (string.len() + 1).try_into().unwrap();
    // NSData will handle releasing the string (it is autoreleased)
    let _: id = msg_class![env; NSData dataWithBytesNoCopy:c_string
                                                    length:length];
    c_string
}

// These come from a category in UIKit (UIStringDrawing).
// TODO: Implement categories so we can completely move the code to UIFont.
// TODO: More `sizeWithFont:` variants
- (CGSize)sizeWithFont:(id)font { // UIFont*
    // TODO: avoid copy
    let text = to_rust_string(env, this);
    ui_font::size_with_font(env, font, &text, None)
}
- (CGSize)sizeWithFont:(id)font // UIFont*
     constrainedToSize:(CGSize)size {
    msg![env; this sizeWithFont:font
              constrainedToSize:size
                  lineBreakMode:UILineBreakModeWordWrap]
}
- (CGSize)sizeWithFont:(id)font // UIFont*
     constrainedToSize:(CGSize)size
         lineBreakMode:(UILineBreakMode)line_break_mode {
    // TODO: avoid copy
    let text = to_rust_string(env, this);
    ui_font::size_with_font(env, font, &text, Some((size, line_break_mode)))
}

- (CGSize)drawInRect:(CGRect)rect
            withFont:(id)font { // UIFont*
    msg![env; this drawInRect:rect
                     withFont:font
                lineBreakMode:UILineBreakModeWordWrap
                    alignment:UITextAlignmentLeft]
}
- (CGSize)drawInRect:(CGRect)rect
            withFont:(id)font // UIFont*
       lineBreakMode:(UILineBreakMode)line_break_mode {
    msg![env; this drawInRect:rect
                     withFont:font
                lineBreakMode:line_break_mode
                    alignment:UITextAlignmentLeft]
}
- (CGSize)drawInRect:(CGRect)rect
            withFont:(id)font // UIFont*
       lineBreakMode:(UILineBreakMode)line_break_mode
           alignment:(UITextAlignment)align {
    // TODO: avoid copy
    let text = to_rust_string(env, this);
    ui_font::draw_in_rect(env, font, &text, rect, line_break_mode, align)
}

@end

// Our private subclass that is the single implementation of NSString for the
// time being.
@implementation _touchHLE_NSString: NSString

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(StringHostObject::Utf8(Cow::Borrowed("")));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: more init methods

- (id)initWithBytes:(ConstPtr<u8>)bytes
             length:(NSUInteger)len
           encoding:(NSStringEncoding)encoding {
    assert!(encoding == NSUTF8StringEncoding); // TODO: other encodings

    // TODO: error handling
    let slice = env.mem.bytes_at(bytes, len);
    let host_object = StringHostObject::decode(Cow::Borrowed(slice), encoding);

    *env.objc.borrow_mut(this) = host_object;

    this
}

- (id)initWithCString:(ConstPtr<u8>)c_string {
    // This is a deprecated method nobody should use, but unfortunately, it is
    // used. The encoding it should use is [NSString defaultCStringEncoding]
    // but I don't want to figure out what that is on all platforms, and the use
    // I've seen of this method was on ASCII strings, so let's just hardcode
    // UTF-8 and hope that works.
    msg![env; this initWithCString:c_string encoding:NSUTF8StringEncoding]
}

- (id)initWithCString:(ConstPtr<u8>)c_string
             encoding:(NSStringEncoding)encoding {
    assert!(encoding != NSUTF16StringEncoding);
    let len: NSUInteger = env.mem.cstr_at(c_string).len().try_into().unwrap();
    msg![env; this initWithBytes:c_string length:len encoding:encoding]
}

- (id)initWithContentsOfFile:(id)path // NSString*
                    encoding:(NSStringEncoding)encoding
                       error:(MutPtr<id>)error { // NSError**
    assert!(error.is_null()); // TODO: error handling

    // TODO: avoid copy?
    let path = to_rust_string(env, path);
    let bytes = env.fs.read(GuestPath::new(&path)).unwrap();

    let host_object = StringHostObject::decode(Cow::Owned(bytes), encoding);

    *env.objc.borrow_mut(this) = host_object;

    this
}

- (id)initWithUTF8String:(ConstPtr<u8>)c_string {
    msg![env; this initWithCString:c_string encoding:NSUTF8StringEncoding]
}

@end

// Specialised subclass for static-lifetime strings.
// See `get_static_str`.
@implementation _touchHLE_NSString_Static: _touchHLE_NSString

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(StringHostObject::Utf8(Cow::Borrowed("")));
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id) retain { this }
- (()) release {}
- (id) autorelease { this }

@end

// Specialised subclasses for static-lifetime strings from the guest app binary.
@implementation _touchHLE_NSString_CFConstantString_UTF8: _touchHLE_NSString_Static

- (ConstPtr<u8>)UTF8String {
    let cfstringStruct { bytes, .. } = env.mem.read(this.cast());

    bytes
}

@end

@implementation _touchHLE_NSString_CFConstantString_UTF16: _touchHLE_NSString_Static
@end

};

/// For use by [crate::dyld]: Handle static strings listed in the app binary.
/// Sets up host objects and updates `isa` fields
/// (`___CFConstantStringClassReference` is ignored by our dyld).
pub fn register_constant_strings(bin: &MachO, mem: &mut Mem, objc: &mut ObjC) {
    let Some(cfstrings) = bin.get_section("__cfstring") else { return; };

    assert!(cfstrings.size % guest_size_of::<cfstringStruct>() == 0);
    let base: ConstPtr<cfstringStruct> = Ptr::from_bits(cfstrings.addr);
    for i in 0..(cfstrings.size / guest_size_of::<cfstringStruct>()) {
        let cfstr_ptr = base + i;
        let cfstringStruct {
            _isa,
            flags,
            bytes,
            length,
        } = mem.read(cfstr_ptr);

        // Constant CFStrings should (probably) only ever have flags 0x7c8 and
        // 0x7d0.
        // See https://lists.llvm.org/pipermail/cfe-dev/2008-August/002518.html
        let (host_object, class_name) = if flags == 0x7C8 {
            // ASCII
            let decoded = std::str::from_utf8(mem.bytes_at(bytes, length)).unwrap();

            (
                StringHostObject::Utf8(Cow::Owned(String::from(decoded))),
                "_touchHLE_NSString_CFConstantString_UTF8",
            )
        } else if flags == 0x7D0 {
            // UTF16 (length is in code units, not bytes)
            let decoded = mem
                .bytes_at(bytes, length * 2)
                .chunks(2)
                .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
                .collect();

            (
                StringHostObject::Utf16(decoded),
                "_touchHLE_NSString_CFConstantString_UTF16",
            )
        } else {
            panic!("Bad CFTypeID for constant string: {:#x}", flags);
        };

        objc.register_static_object(cfstr_ptr.cast().cast_mut(), Box::new(host_object));

        let new_isa = objc.get_known_class(class_name, mem);
        mem.write(cfstr_ptr.cast().cast_mut(), new_isa);
    }
}

/// Shortcut for host code: get an NSString corresponding to a `&'static str`,
/// which does not have to be released and is never deallocated.
pub fn get_static_str(env: &mut Environment, from: &'static str) -> id {
    if let Some(&existing) = State::get(env).static_str_pool.get(from) {
        existing
    } else {
        let new = msg_class![env; _touchHLE_NSString_Static alloc];
        *env.objc.borrow_mut(new) = StringHostObject::Utf8(Cow::Borrowed(from));
        State::get(env).static_str_pool.insert(from, new);
        new
    }
}

/// Shortcut for host code, roughly equivalent to
/// `[[NSString alloc] initWithUTF8String:]` in the proper API.
pub fn from_rust_string(env: &mut Environment, from: String) -> id {
    let string: id = msg_class![env; _touchHLE_NSString alloc];
    let host_object: &mut StringHostObject = env.objc.borrow_mut(string);
    *host_object = StringHostObject::Utf8(Cow::Owned(from));
    string
}

/// Shortcut for host code, provides a view of a string in UTF-8.
/// Warning: This may panic if the string is not valid UTF-16!
///
/// TODO: Try to avoid allocating a new String in more cases.
///
/// TODO: Try to avoid converting from UTF-16 in more cases.
pub fn to_rust_string(env: &mut Environment, string: id) -> Cow<'static, str> {
    // TODO: handle foreign subclasses of NSString
    env.objc
        .borrow_mut::<StringHostObject>(string)
        .to_utf8()
        .unwrap()
}

/// Shortcut for host code, calls a callback once for each UTF-16 code-unit in a
/// string. This is equivalent to a for loop using the `length` and
/// `characterAtIndex:` methods, but much more efficient.
pub fn for_each_code_unit<F>(env: &mut Environment, string: id, mut f: F)
where
    F: FnMut(NSUInteger, u16),
{
    // TODO: handle foreign subclasses of NSString
    let mut idx: NSUInteger = 0;
    env.objc
        .borrow::<StringHostObject>(string)
        .iter_code_units()
        .for_each(|c| {
            f(idx, c);
            idx += 1;
        });
}
