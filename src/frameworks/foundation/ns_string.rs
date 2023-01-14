//! The `NSString` class cluster, including `NSMutableString`.

use super::ns_array;
use super::NSUInteger;
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, Mem, MutPtr, MutVoidPtr, SafeRead};
use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, retain, Class, ClassExports, HostObject, ObjC,
};
use crate::Environment;
use std::borrow::Cow;
use std::collections::HashMap;
use std::string::FromUtf16Error;

pub type NSStringEncoding = NSUInteger;
pub const NSUTF8StringEncoding: NSUInteger = 4;
pub const NSUnicodeStringEncoding: NSUInteger = 10;
pub const NSUTF16StringEncoding: NSUInteger = NSUnicodeStringEncoding;

#[derive(Default)]
pub struct State {
    static_str_pool: HashMap<&'static str, id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.foundation.ns_string
    }
}

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

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSString is an abstract class. A subclass must provide:
// - (NSUInteger)length;
// - (unichar)characterAtIndex:(NSUInteger)index;
// We can pick whichever subclass we want for the various alloc methods.
// For the time being, that will always be _touchHLE_NSString.
@implementation NSString: NSObject

+ (id)allocWithZone:(MutVoidPtr)zone {
    // NSString might be subclassed by something which needs allocWithZone:
    // to have the normal behaviour. Unimplemented: call superclass alloc then.
    assert!(this == env.objc.get_known_class("NSString", &mut env.mem));
    msg_class![env; _touchHLE_NSString allocWithZone:zone]
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

// NSCopying implementation
- (id)copyWithZone:(MutVoidPtr)_zone {
    // TODO: override this once we have NSMutableString!
    retain(env, this)
}

- (bool)getCString:(MutPtr<u8>)buffer
         maxLength:(NSUInteger)buffer_size
          encoding:(NSStringEncoding)encoding {
    assert!(encoding == NSUTF8StringEncoding); // TODO: other encodings

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
    'outer: loop {
        // attempt to match a separator
        {
            let mut main_match = main_iter.clone();
            let mut sep_match = sep_iter.clone();
            'inner: loop {
                match sep_match.next() {
                    None => {
                        components.push(std::mem::take(&mut current_component));
                        main_iter = main_match;
                        continue 'outer;
                    },
                    Some(sep_c) => {
                        let main_c = main_match.next();
                        if main_c != Some(sep_c) {
                            break 'inner;
                        }
                    }
                }
            }
        }

        // no separator match, extend the current component
        match main_iter.next() {
            Some(cur) => current_component.push(cur),
            None => break,
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

@end

// Our private subclass that is the single implementation of NSString for the
// time being.
@implementation _touchHLE_NSString: NSString

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(StringHostObject::Utf8(Cow::Borrowed("")));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: accessors, more init methods, etc

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

@end

// Specialised subclass for static-lifetime strings.
// See `get_static_str`.
@implementation _touchHLE_NSString_Static: _touchHLE_NSString

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(StringHostObject::Utf8(Cow::Borrowed("")));
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id) retain { this }
- (()) release {}
- (id) autorelease { this }

@end

// Specialised subclass for static-lifetime strings from the guest app binary.
// (This may be useful eventually for efficiently implementing accessors that
// provide a pointer to the string bytes.)
@implementation _touchHLE_NSString_CFConstantString: _touchHLE_NSString_Static
@end

};

/// For use by [crate::dyld]: Handle a static string found in the app binary
/// (`isa` = `___CFConstantStringClassReference`). Set up the correct host
/// object and return the `isa` to be written.
pub fn handle_constant_string(mem: &mut Mem, objc: &mut ObjC, constant_str: id) -> Class {
    // Ghidra calls it this. The field names and types are guesswork.
    #[allow(non_camel_case_types)]
    struct cfstringStruct {
        _isa: Class,
        flags: u32,
        bytes: ConstPtr<u8>,
        length: NSUInteger,
    }
    unsafe impl SafeRead for cfstringStruct {}

    let cfstringStruct {
        _isa,
        flags,
        bytes,
        length,
    } = mem.read(constant_str.cast());
    assert!(flags == 0x7C8); // no idea what this means

    // All the strings I've seen are ASCII, so this might be wrong.
    let decoded = std::str::from_utf8(mem.bytes_at(bytes, length)).unwrap();

    let host_object = StringHostObject::Utf8(Cow::Owned(String::from(decoded)));

    objc.register_static_object(constant_str, Box::new(host_object));

    objc.get_known_class("_touchHLE_NSString_CFConstantString", mem)
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
