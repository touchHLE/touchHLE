/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! UIKit NIB Archives decoder. This is _not_ a part of public API!
//!
//! Resources:
//! - [UIKit NIB Archives](https://www.mothersruin.com/software/Archaeology/reverse/uinib.html)
//!
//! - [NibArchive File Format](https://github.com/matsmattsson/nibsqueeze/blob/master/NibArchive.md)

use crate::environment::Environment;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::mem::ConstVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use nibarchive::{NIBArchive, Value, ValueVariant};

#[derive(Default)]
struct NIBArchiveDecoderHostObject {
    /// `None` only for the phantom-default returned via `Default::default()`
    /// when the objc runtime needs a placeholder for a missing/wrong-typed
    /// host object. Real archives are always loaded via
    /// `_touchHLE_initForReadingWithData:`.
    archive: Option<NIBArchive>,
    current_object_idx: Option<u32>,
    /// linear map of idx => id
    already_unarchived: Vec<Option<id>>,
    delegate: id,
}
impl HostObject for NIBArchiveDecoderHostObject {}

/// NIB archive's binary value marker for float-encoded numbers (4 bytes each
/// in little-endian). This is the canonical encoding used by UIKit when it
/// archives a CGFloat/CGPoint/CGRect on a 32-bit ABI.
const NIB_MARKER_FLOAT: u8 = 6;
/// Same as [`NIB_MARKER_FLOAT`] but with double-precision (8 bytes each in
/// little-endian). Used by newer UIKit versions and Mac archives.
const NIB_MARKER_DOUBLE: u8 = 7;

/// Best-effort decode of a NIB-archived `CGPoint`. Returns `None` if the data
/// doesn't match any of the known encodings; the caller is then expected to
/// log a warning and return `CGPointZero`, matching how Apple's runtime
/// silently substitutes zero when an archived value is corrupt.
fn decode_nib_cgpoint(data: &[u8]) -> Option<CGPoint> {
    match data.first().copied()? {
        NIB_MARKER_FLOAT if data.len() >= 9 => Some(CGPoint {
            x: f32::from_le_bytes(data[1..5].try_into().ok()?),
            y: f32::from_le_bytes(data[5..9].try_into().ok()?),
        }),
        NIB_MARKER_DOUBLE if data.len() >= 17 => Some(CGPoint {
            x: f64::from_le_bytes(data[1..9].try_into().ok()?) as f32,
            y: f64::from_le_bytes(data[9..17].try_into().ok()?) as f32,
        }),
        _ => None,
    }
}

/// Best-effort decode of a NIB-archived `CGRect`. Returns `None` if the data
/// doesn't match any of the known encodings; see [`decode_nib_cgpoint`] for
/// the caller contract.
fn decode_nib_cgrect(data: &[u8]) -> Option<CGRect> {
    match data.first().copied()? {
        NIB_MARKER_FLOAT if data.len() >= 17 => Some(CGRect {
            origin: CGPoint {
                x: f32::from_le_bytes(data[1..5].try_into().ok()?),
                y: f32::from_le_bytes(data[5..9].try_into().ok()?),
            },
            size: CGSize {
                width: f32::from_le_bytes(data[9..13].try_into().ok()?),
                height: f32::from_le_bytes(data[13..17].try_into().ok()?),
            },
        }),
        NIB_MARKER_DOUBLE if data.len() >= 33 => Some(CGRect {
            origin: CGPoint {
                x: f64::from_le_bytes(data[1..9].try_into().ok()?) as f32,
                y: f64::from_le_bytes(data[9..17].try_into().ok()?) as f32,
            },
            size: CGSize {
                width: f64::from_le_bytes(data[17..25].try_into().ok()?) as f32,
                height: f64::from_le_bytes(data[25..33].try_into().ok()?) as f32,
            },
        }),
        _ => None,
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_NIBArchiveDecoder: NSCoder

+ (id)allocWithZone:(NSZonePtr)_zone { // struct _NSZone*
    let unarchiver = Box::new(NIBArchiveDecoderHostObject {
        archive: Some(NIBArchive::new_unchecked(
            Default::default(), Default::default(), Default::default(), Default::default()
        )),
        current_object_idx: None,
        already_unarchived: Vec::new(),
        delegate: nil,
    });
    env.objc.alloc_object(this, unarchiver, &mut env.mem)
}

- (id)_touchHLE_initForReadingWithData:(id)data {
    if data == nil {
        return nil;
    }

    let length: NSUInteger = msg![env; data length];
    let bytes: ConstVoidPtr = msg![env; data bytes];
    // The data comes from the guest's application bundle which means it can be
    // corrupt, truncated, or even encrypted (FairPlay). Real Foundation returns
    // nil from -[NSCoder initForReadingWithData:] in that case; do the same so
    // the game can fall back to its own error handling instead of crashing the
    // emulator.
    let archive: NIBArchive = match NIBArchive::from_bytes(env.mem.bytes_at(bytes.cast(), length)) {
        Ok(a) => a,
        Err(e) => {
            log!(
                "Warning: -[NSCoder initForReadingWithData:] failed to parse {} \
                 byte NIB archive: {:?}; returning nil.",
                length, e
            );
            release(env, this);
            return nil;
        }
    };

    // for (i, object) in archive.objects().iter().enumerate() {
    //     let class_name = object.class_name(archive.class_names()).name();
    //     println!("[{i}] Object of a class '{class_name}':");
    //
    //     let values: &[nibarchive::Value] = object.values(archive.values());
    //     for (j, value) in values.iter().enumerate() {
    //         let key = value.key(archive.keys());
    //         let inner_value = value.value();
    //         println!("-- [{j}] {key}: {inner_value:?}");
    //     }
    // }

    let host_obj = env.objc.borrow_mut::<NIBArchiveDecoderHostObject>(this);
    assert!(host_obj.already_unarchived.is_empty());
    assert!(host_obj.current_object_idx.is_none());
    let objects_count = archive.objects().len();

    host_obj.already_unarchived = vec![None; objects_count];
    host_obj.archive = Some(archive);
    // we start from the 'top'
    host_obj.current_object_idx = Some(0);
    this
}

- (())dealloc {
    let host_obj: &mut NIBArchiveDecoderHostObject = env.objc.borrow_mut(this);
    let already_unarchived = std::mem::take(&mut host_obj.already_unarchived);
    for &object in already_unarchived.iter().flatten() {
        release(env, object);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: implement calls to delegate methods
// weak/non-retaining
- (())setDelegate:(id)delegate { // id<?>
    let host_object = env.objc.borrow_mut::<NIBArchiveDecoderHostObject>(this);
    host_object.delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<NIBArchiveDecoderHostObject>(this).delegate
}

// These methods drive most of the decoding.
// They get called in two cases:
// - By the code that initiates the unarchival, e.g. UINib, to retrieve
//   top-level objects.
// - By the object currently being unarchived, i.e. something that had
//   `initWithCoder:` called on it, to retrieve objects from its scope.
// They are all from the NSCoder abstract class and they return default values
// if the key is unknown.

- (bool)decodeBoolForKey:(id)key { // NSString *
    get_value_to_decode_for_key(env, this, key).is_some_and(|value| {
        match value.value() {
            &ValueVariant::Bool(b) => b,
            // Some NIB encoders store booleans as small integers. Match Apple's
            // -decodeBoolForKey: behaviour of treating any non-zero number as
            // true and warn loudly instead of crashing on unexpected variants.
            ValueVariant::Int8(i) => *i != 0,
            ValueVariant::Int16(i) => *i != 0,
            ValueVariant::Int32(i) => *i != 0,
            ValueVariant::Int64(i) => *i != 0,
            other => {
                log!(
                    "Warning: decodeBoolForKey expected a bool, got {:?}; \
                     returning false.",
                    other
                );
                false
            }
        }
    })
}

- (f32)decodeFloatForKey:(id)key { // NSString *
    get_value_to_decode_for_key(env, this, key).map_or(
        0.0,
        |value| {
            match value.value() {
                ValueVariant::Float(f) => *f,
                ValueVariant::Double(d) => *d as f32,
                ValueVariant::Int8(i) => *i as f32,
                ValueVariant::Int16(i) => *i as f32,
                ValueVariant::Int32(i) => *i as f32,
                ValueVariant::Int64(i) => *i as f32,
                _ => {
                    log!("Warning: decodeFloatForKey expected a number, got unexpected variant");
                    0.0
                }
            }
        }
    )
}

- (NSInteger)decodeIntegerForKey:(id)key { // NSString *
    // Bounds-clamp instead of raising NSRangeException; real iOS would raise on
    // overflow but games that hit this typically just crash anyway, so we'd
    // rather log + degrade than match that behaviour precisely.
    get_value_to_decode_for_key(env, this, key).map_or(
        0,
        |value| {
            match value.value() {
                ValueVariant::Bool(b) => if *b { 1 } else { 0 },
                ValueVariant::Int8(i) => (*i).into(),
                ValueVariant::Int16(i) => (*i).into(),
                ValueVariant::Int32(i) => *i,
                ValueVariant::Int64(i) => {
                    let i = *i;
                    if i > NSInteger::MAX as i64 {
                        NSInteger::MAX
                    } else if i < NSInteger::MIN as i64 {
                        NSInteger::MIN
                    } else {
                        i as NSInteger
                    }
                }
                ValueVariant::Float(f) => *f as NSInteger,
                ValueVariant::Double(d) => *d as NSInteger,
                other => {
                    log!(
                        "Warning: decodeIntegerForKey expected a number, got \
                         {:?}; returning 0.",
                        other
                    );
                    0
                }
            }
        }
    )
}
- (i32)decodeIntForKey:(id)key { // NSString *
    msg![env; this decodeIntegerForKey:key]
}


- (id)decodeObjectForKey:(id)key { // NSString*
    let Some(val) = get_value_to_decode_for_key(env, this, key) else {
        return nil;
    };
    let idx = match val.value() {
        &ValueVariant::ObjectRef(idx) => idx,
        // The encoder is allowed to inline a nil reference as a Nil variant. We
        // also see games whose NIBs store non-object data under "object" keys
        // when they were generated by buggy IB plugins; treat any non-ObjectRef
        // as nil instead of crashing.
        other => {
            log!(
                "Warning: decodeObjectForKey expected an object ref, got \
                 {:?}; returning nil.",
                other
            );
            return nil;
        }
    };
    let object = unarchive_obj(env, this, idx);

    // on behalf of the caller
    retain(env, object);
    autorelease(env, object)
}

- (bool)containsValueForKey:(id)key { // NSString*
    if key == nil {
        return false;
    }
    get_value_to_decode_for_key(env, this, key).is_some()
}

- (CGPoint)decodeCGPointForKey:(id)key { // NSString*
    let val = get_value_to_decode_for_key(env, this, key);
    let Some(val) = val else {
        return CGPoint { x: 0.0, y: 0.0 };
    };
    let ValueVariant::Data(data) = val.value() else {
        log!(
            "Warning: decodeCGPointForKey: expected Data variant, got \
             {:?}; returning CGPointZero.",
            val.value()
        );
        return CGPoint { x: 0.0, y: 0.0 };
    };
    if let Some(point) = decode_nib_cgpoint(data) {
        // Copy out of the packed struct before formatting to avoid taking
        // a reference to a possibly-misaligned field.
        let (x, y) = (point.x, point.y);
        log_dbg!("decoded CGPoint {} {}", x, y);
        point
    } else {
        log!(
            "Warning: -[NSCoder decodeCGPointForKey:] got unrecognised NIB data \
             (first byte = 0x{:02x}, len = {}); returning CGPointZero",
            data.first().copied().unwrap_or(0),
            data.len()
        );
        CGPoint { x: 0.0, y: 0.0 }
    }
}

- (CGRect)decodeCGRectForKey:(id)key { // NSString*
    let val = get_value_to_decode_for_key(env, this, key);
    let Some(val) = val else {
        return CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 0.0, height: 0.0 }
        };
    };
    let ValueVariant::Data(data) = val.value() else {
        log!(
            "Warning: decodeCGRectForKey: expected Data variant, got \
             {:?}; returning CGRectZero.",
            val.value()
        );
        return CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 0.0, height: 0.0 }
        };
    };
    if let Some(rect) = decode_nib_cgrect(data) {
        // Copy out of the packed struct before formatting to avoid taking
        // a reference to a possibly-misaligned field.
        let (x, y) = (rect.origin.x, rect.origin.y);
        let (w, h) = (rect.size.width, rect.size.height);
        log_dbg!("decoded CGRect {} {} {} {}", x, y, w, h);
        rect
    } else {
        log!(
            "Warning: -[NSCoder decodeCGRectForKey:] got unrecognised NIB data \
             (first byte = 0x{:02x}, len = {}); returning CGRectZero",
            data.first().copied().unwrap_or(0),
            data.len()
        );
        CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 0.0, height: 0.0 },
        }
    }
}

@end

};

fn borrow_host_obj(env: &mut Environment, unarchiver: id) -> &mut NIBArchiveDecoderHostObject {
    env.objc.borrow_mut(unarchiver)
}

fn get_value_to_decode_for_key(env: &mut Environment, unarchiver: id, key: id) -> Option<&Value> {
    let key = to_rust_string(env, key);
    // TODO: avoid copying string
    let host_obj = borrow_host_obj(env, unarchiver);
    // `current_object_idx` is set by `_touchHLE_initForReadingWithData:` and
    // always restored by `unarchive_obj` after a nested decode, so it should
    // be `Some` whenever a guest calls a decode method. Be defensive against
    // a malformed archive that somehow leaves it cleared.
    let current_idx = host_obj.current_object_idx?;
    let obj = host_obj.archive.as_ref().unwrap().objects().get(current_idx as usize)?;
    obj.values(host_obj.archive.as_ref().unwrap().values())
        .iter()
        .find(|&val| val.key(host_obj.archive.as_ref().unwrap().keys()) == &key)
        .map(|v| v as _)
}

fn unarchive_obj(env: &mut Environment, unarchiver: id, idx: u32) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    let idx_usize = idx as usize;
    // Bounds check: a malformed NIB can reference an object index that is
    // out of range. Return nil instead of crashing.
    if idx_usize >= host_obj.already_unarchived.len() {
        log!(
            "Warning: NIB archive references out-of-range object index {} \
             (max {}); returning nil.",
            idx,
            host_obj.already_unarchived.len()
        );
        return nil;
    }
    if let Some(existing) = host_obj.already_unarchived[idx_usize] {
        return existing;
    }

    let Some(object) = host_obj.archive.as_ref().unwrap().objects().get(idx_usize) else {
        log!(
            "Warning: NIB archive has fewer objects than indices indicate at \
             idx {}; returning nil.",
            idx
        );
        return nil;
    };
    // Resolve the "best available" class for this NIB entry. Apple's
    // NIB archive format (used by both legacy XIBs and iOS 5+ storyboards)
    // attaches an ordered list of *fallback class names* to each entry, so
    // that a designer can ship a NIB referencing e.g. `_UIVisualEffectView`
    // but still work on older iOS versions that only know `UIView`. The
    // runtime's documented behaviour (see Apple's
    // <https://developer.apple.com/documentation/uikit/uinib> and
    // <https://developer.apple.com/documentation/foundation/nscoder>) is:
    //
    //   1. Try the primary class name.
    //   2. If `NSClassFromString(primary)` is nil, walk the fallbacks in
    //      order and use the first one that resolves.
    //   3. If everything fails, install a placeholder and keep going.
    //
    // Modern Xcode emits NIBs where the primary `name` field is *empty*
    // when the class is purely opt-in / iOS-version-gated; everything that
    // matters is in the fallback list. Previously we just unconditionally
    // looked up the primary name, which meant we tried to instantiate the
    // class literally called `""` and printed the
    // `Warning: get_known_class called with a garbage/empty class name ("")`
    // message users see on storyboard apps. Honour the fallbacks list
    // instead.
    // First snapshot the primary + fallback names out of the archive so we
    // can drop the `host_obj` borrow before calling into `env.objc`.
    let (primary_class_name, fallback_class_names): (String, Vec<String>) = {
        let class_name_entry = object.class_name(host_obj.archive.as_ref().unwrap().class_names());
        let primary = class_name_entry.name().to_string();
        let fallbacks: Vec<String> = class_name_entry
            .fallback_classes(host_obj.archive.as_ref().unwrap().class_names())
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        (primary, fallbacks)
    };
    // Walk the candidate list and pick the first name that's actually
    // registered / has a host implementation. If nothing resolves, pick
    // the first non-empty name we have so the placeholder warning is at
    // least usable; only fall through to the (possibly empty) primary as
    // a last resort.
    let chosen_class_name: String = {
        let mut chosen: Option<String> = None;
        let candidates: Vec<&str> = std::iter::once(primary_class_name.as_str())
            .chain(fallback_class_names.iter().map(|s| s.as_str()))
            .filter(|s| !s.is_empty())
            .collect();
        for name in &candidates {
            if env.objc.try_get_known_class(name, &mut env.mem).is_some() {
                chosen = Some(name.to_string());
                break;
            }
        }
        chosen
            .or_else(|| candidates.first().map(|s| s.to_string()))
            .unwrap_or_else(|| primary_class_name.clone())
    };
    log_dbg!(
        "Unarchiving object at index {} (primary class '{}', fallbacks {:?}, chosen '{}')",
        idx,
        primary_class_name,
        fallback_class_names,
        chosen_class_name
    );
    if chosen_class_name.is_empty() {
        log!(
            "Warning: nib archive decoder: empty class name for object {} (primary {:?}, fallbacks {:?}).",
            idx,
            primary_class_name,
            fallback_class_names
        );
    }
    let class = env.objc.get_known_class(&chosen_class_name, &mut env.mem);

    let host_obj = borrow_host_obj(env, unarchiver);
    // reborrow
    let old_current_idx = host_obj.current_object_idx;
    host_obj.current_object_idx = Some(idx);

    let new_object: id = msg![env; class alloc];
    let new_object: id = msg![env; new_object initWithCoder:unarchiver];

    let host_obj = borrow_host_obj(env, unarchiver); // reborrow
    host_obj.current_object_idx = old_current_idx;
    let host_obj = borrow_host_obj(env, unarchiver); // reborrow
    host_obj.already_unarchived[idx_usize] = Some(new_object);
    new_object
}

pub fn decode_current_array(env: &mut Environment, unarchiver: id) -> Vec<id> {
    let host_obj = borrow_host_obj(env, unarchiver);
    let Some(current_idx) = host_obj.current_object_idx else {
        log!("Warning: decode_current_array called outside of unarchival flow.");
        return Vec::new();
    };

    let mut indicies = vec![];

    let Some(object) = host_obj.archive.as_ref().unwrap().objects().get(current_idx as usize) else {
        log!(
            "Warning: decode_current_array: current_object_idx {} is out of \
             range; returning empty array.",
            current_idx
        );
        return Vec::new();
    };
    let values: &[Value] = object.values(host_obj.archive.as_ref().unwrap().values());
    for (i, value) in values.iter().enumerate() {
        let key = value.key(host_obj.archive.as_ref().unwrap().keys());
        let inner_value = value.value();
        if i == 0 {
            if key != "NSInlinedValue" {
                log!(
                    "Warning: decode_current_array: first key was {:?}, \
                     expected NSInlinedValue; skipping.",
                    key
                );
                continue;
            }
            // Non-inlined encodings are theoretically possible but we have
            // never seen one in the wild; warn and treat them as empty.
            if !matches!(inner_value, ValueVariant::Bool(true)) {
                log!(
                    "Warning: decode_current_array: NSInlinedValue was {:?}; \
                     bailing out with empty array.",
                    inner_value
                );
                return Vec::new();
            }
            continue;
        }
        if key != "UINibEncoderEmptyKey" {
            log!(
                "Warning: decode_current_array: unexpected key {:?}; skipping.",
                key
            );
            continue;
        }
        let &ValueVariant::ObjectRef(next_idx) = inner_value else {
            log!(
                "Warning: decode_current_array: expected ObjectRef, got \
                 {:?}; skipping.",
                inner_value
            );
            continue;
        };
        indicies.push(next_idx);
    }

    let mut array: Vec<id> = vec![];
    for next_idx in indicies {
        let host_obj = borrow_host_obj(env, unarchiver);
        // reborrow
        let old_current_idx = host_obj.current_object_idx;
        host_obj.current_object_idx = Some(next_idx);
        let next_obj = unarchive_obj(env, unarchiver, next_idx);
        retain(env, next_obj);
        array.push(next_obj);

        let host_obj = borrow_host_obj(env, unarchiver);
        // reborrow
        host_obj.current_object_idx = old_current_idx;
    }
    array
}

pub fn decode_current_dict(env: &mut Environment, unarchiver: id) -> Vec<(id, id)> {
    // Trick: dict is an array of key-value-key-value sequence
    let array = decode_current_array(env, unarchiver);
    // Note: we need to release as `dict_from_keys_and_objects`
    // would retain them later
    for &obj in &array {
        release(env, obj);
    }
    array
        .chunks_exact(2)
        .map(|chunks| (chunks[0], chunks[1]))
        .collect()
}

pub fn decode_current_string(env: &mut Environment, unarchiver: id) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    let Some(current_idx) = host_obj.current_object_idx else {
        log!("Warning: decode_current_string called outside of unarchival flow.");
        return from_rust_string(env, String::new());
    };

    let Some(object) = host_obj.archive.as_ref().unwrap().objects().get(current_idx as usize) else {
        log!(
            "Warning: decode_current_string: current_object_idx {} is out of \
             range; returning empty string.",
            current_idx
        );
        return from_rust_string(env, String::new());
    };
    let values: &[Value] = object.values(host_obj.archive.as_ref().unwrap().values());

    // Find the NS.bytes entry; some NSStrings also include NS.cstring or
    // NS.string variants. Fall back to empty string on anything unexpected.
    let mut bytes_value: Option<&Value> = None;
    for v in values {
        if v.key(host_obj.archive.as_ref().unwrap().keys()) == "NS.bytes" {
            bytes_value = Some(v);
            break;
        }
    }
    let Some(value) = bytes_value else {
        log!(
            "Warning: decode_current_string: no NS.bytes entry (found {} \
             values); returning empty string.",
            values.len()
        );
        return from_rust_string(env, String::new());
    };
    let ValueVariant::Data(inner_data) = value.value() else {
        log!(
            "Warning: decode_current_string: NS.bytes was not a Data \
             variant (got {:?}); returning empty string.",
            value.value()
        );
        return from_rust_string(env, String::new());
    };
    // NIB strings are usually valid UTF-8 but we have encountered broken
    // encoders that pad with arbitrary bytes. `from_utf8_lossy` keeps the
    // game running rather than crashing the emulator on a single bad byte.
    let str = String::from_utf8_lossy(inner_data).into_owned();
    log_dbg!("decode_current_string {str}");
    from_rust_string(env, str)
}

pub fn decode_current_number(env: &mut Environment, unarchiver: id) -> id {
    let host_obj = borrow_host_obj(env, unarchiver);
    let Some(current_idx) = host_obj.current_object_idx else {
        log!("Warning: decode_current_number called outside of unarchival flow.");
        let num = msg_class![env; NSNumber alloc];
        return msg![env; num initWithInteger:(0 as NSInteger)];
    };

    let Some(object) = host_obj.archive.as_ref().unwrap().objects().get(current_idx as usize) else {
        log!(
            "Warning: decode_current_number: current_object_idx {} is out of \
             range; returning 0.",
            current_idx
        );
        let num = msg_class![env; NSNumber alloc];
        return msg![env; num initWithInteger:(0 as NSInteger)];
    };
    let values: &[Value] = object.values(host_obj.archive.as_ref().unwrap().values());
    if values.is_empty() {
        log!("Warning: decode_current_number: empty values; returning 0.");
        let num = msg_class![env; NSNumber alloc];
        return msg![env; num initWithInteger:(0 as NSInteger)];
    }

    let value = &values[0];
    let key = value.key(host_obj.archive.as_ref().unwrap().keys()).clone();
    match key.as_str() {
        "NS.intval" => {
            let ns_key: id = get_static_str(env, "NS.intval");
            let int: NSInteger = msg![env; unarchiver decodeIntegerForKey:ns_key];
            release(env, ns_key);
            let num = msg_class![env; NSNumber alloc];
            msg![env;
            num initWithInteger:int]
        }
        // Apple's NSCFNumber/NSNumber serializer can store doubles, floats,
        // and 64-bit integers under "NS.dblval", "NS.fltval", or NS.special
        // shaped keys. Surface those as best-effort double NSNumbers so we
        // don't hard-crash on any non-int.
        "NS.dblval" => {
            let ns_key: id = get_static_str(env, "NS.dblval");
            let d: f64 = msg![env; unarchiver decodeDoubleForKey:ns_key];
            release(env, ns_key);
            let num = msg_class![env; NSNumber alloc];
            msg![env; num initWithDouble:d]
        }
        "NS.special" => {
            let ns_key: id = get_static_str(env, "NS.special");
            let d: f64 = msg![env; unarchiver decodeDoubleForKey:ns_key];
            release(env, ns_key);
            let num = msg_class![env; NSNumber alloc];
            msg![env; num initWithDouble:d]
        }
        "NS.fltval" => {
            let ns_key: id = get_static_str(env, "NS.fltval");
            let f: f32 = msg![env; unarchiver decodeFloatForKey:ns_key];
            release(env, ns_key);
            let num = msg_class![env; NSNumber alloc];
            msg![env; num initWithFloat:f]
        }
        _ => {
            log!(
                "Warning: decode_current_number: unknown key {:?}; \
                 returning 0.",
                key
            );
            let num = msg_class![env; NSNumber alloc];
            msg![env; num initWithInteger:(0 as NSInteger)]
        }
    }
}
