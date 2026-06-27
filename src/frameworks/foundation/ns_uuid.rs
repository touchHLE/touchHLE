/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSUUID`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, retain, ClassExports, HostObject, NSZonePtr,
};

// =========================================================================
// MARK: - UUID bytes storage
// =========================================================================

/// 16 raw bytes of a UUID (big-endian, RFC 4122 layout).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct UuidBytes(pub [u8; 16]);

impl UuidBytes {
    /// Generate a random version-4 UUID using the system time as a seed.
    /// Not cryptographically secure but good enough for game session IDs.
    pub fn random() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();

        // Simple LCG to fill 16 bytes from the seed.
        let mut state = seed as u64 ^ 0x9e3779b97f4a7c15;
        let mut bytes = [0u8; 16];
        for b in &mut bytes {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (state >> 33) as u8;
        }

        // Set version 4 bits (bits 12-15 of time_hi_and_version = 0100).
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        // Set variant bits (bits 6-7 of clock_seq_hi = 10).
        bytes[8] = (bytes[8] & 0x3F) | 0x80;

        UuidBytes(bytes)
    }

    /// Parse a UUID from a standard string like
    /// `"550E8400-E29B-41D4-A716-446655440000"`.
    pub fn from_string(s: &str) -> Option<Self> {
        let s = s.trim();
        // Accept with or without braces: {xxxxxxxx-...} or xxxxxxxx-...
        let s = s
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .unwrap_or(s);
        // Remove dashes and parse 32 hex digits.
        let hex: String = s.chars().filter(|&c| c != '-').collect();
        if hex.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(UuidBytes(bytes))
    }

    /// Format as `"XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"` (uppercase).
    pub fn to_string(&self) -> String {
        let b = &self.0;
        format!(
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            b[0],  b[1],  b[2],  b[3],
            b[4],  b[5],
            b[6],  b[7],
            b[8],  b[9],
            b[10], b[11], b[12], b[13], b[14], b[15],
        )
    }

    /// Format as lowercase UUID string.
    pub fn to_string_lowercase(&self) -> String {
        self.to_string().to_lowercase()
    }
}

// =========================================================================
// MARK: - Host object
// =========================================================================

#[derive(Default)]
pub struct NSUUIDHostObject {
    pub bytes: UuidBytes,
}
impl HostObject for NSUUIDHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUUID: NSObject

// =========================================================================
// MARK: - Allocation
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSUUIDHostObject {
        bytes: UuidBytes([0u8; 16]),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// =========================================================================
// MARK: - Constructors
// =========================================================================

// ИСПРАВЛЕНИЕ: заменили `///` на `//` внутри макроса, чтобы не генерировался
// атрибут #[doc...]
// `+UUID` — convenience constructor returning a new random UUID.
+ (id)UUID {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    crate::objc::autorelease(env, new)
}

// `-init` — designated initialiser; generates a random UUID.
- (id)init {
    {
        let host = env.objc.borrow_mut::<NSUUIDHostObject>(this);
        host.bytes = UuidBytes::random();
    }
    log_dbg!("NSUUID init => {}", env.objc.borrow::<NSUUIDHostObject>(this).bytes.to_string());
    this
}

// `-initWithUUIDString:` — parses a UUID from a string.
// Returns nil if the string is not a valid UUID.
- (id)initWithUUIDString:(id)string { // NSString*
    if string == nil {
        let _: () = msg![env; this release];
        return nil;
    }
    let s = ns_string::to_rust_string(env, string).into_owned();
    match UuidBytes::from_string(&s) {
        Some(bytes) => {
            {
                let host = env.objc.borrow_mut::<NSUUIDHostObject>(this);
                host.bytes = bytes;
            }
            log_dbg!("NSUUID initWithUUIDString:{:?} => OK", s);
            this
        }
        None => {
            log!("NSUUID initWithUUIDString:{:?} — invalid UUID string, returning nil", s);
            let _: () = msg![env; this release];
            nil
        }
    }
}

// `-initWithUUIDBytes:` — initialises from a 16-byte C array.
- (id)initWithUUIDBytes:(crate::mem::ConstPtr<u8>)bytes_ptr {
    if bytes_ptr.is_null() {
        let _: () = msg![env; this release];
        return nil;
    }
    let mut bytes = [0u8; 16];
    let src = env.mem.bytes_at(bytes_ptr, 16);
    bytes.copy_from_slice(src);
    {
        let host = env.objc.borrow_mut::<NSUUIDHostObject>(this);
        host.bytes = UuidBytes(bytes);
    }
    this
}

// =========================================================================
// MARK: - Accessors
// =========================================================================

// `-UUIDString` — returns the UUID as an uppercase hyphenated NSString*.
- (id)UUIDString { // NSString*
    let s = env.objc.borrow::<NSUUIDHostObject>(this).bytes.to_string();
    let ns = ns_string::from_rust_string(env, s);
    crate::objc::autorelease(env, ns)
}

// `-getUUIDBytes:` — copies the 16 raw bytes into the caller's buffer.
- (())getUUIDBytes:(crate::mem::MutPtr<u8>)out_bytes {
    if out_bytes.is_null() { return; }
    let bytes = env.objc.borrow::<NSUUIDHostObject>(this).bytes.0;
    let dst = env.mem.bytes_at_mut(out_bytes, 16);
    dst.copy_from_slice(&bytes);
}

// =========================================================================
// MARK: - Equality / hashing
// =========================================================================

- (bool)isEqual:(id)other {
    if this == other { return true; }
    if other == nil  { return false; }

    // Check if other is also an NSUUID by looking at its class.
    let uuid_class = env.objc.get_known_class("NSUUID", &mut env.mem);
    let other_class: id = msg![env; other class];
    let is_uuid: bool = msg![env; other_class isSubclassOfClass:uuid_class];
    if !is_uuid { return false; }

    let a = env.objc.borrow::<NSUUIDHostObject>(this).bytes;
    let b = env.objc.borrow::<NSUUIDHostObject>(other).bytes;
    a == b
}

- (u32)hash { // NSUInteger
    let bytes = env.objc.borrow::<NSUUIDHostObject>(this).bytes.0;
    // FNV-1a over the 16 bytes.
    let mut h: u32 = 2166136261;
    for &b in &bytes {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

// =========================================================================
// MARK: - Comparison
// =========================================================================

- (i32)compare:(id)other { // NSComparisonResult
    if other == nil { return 1; } // self > nil
    let a = env.objc.borrow::<NSUUIDHostObject>(this).bytes.to_string();
    let b_uuid_class = env.objc.get_known_class("NSUUID", &mut env.mem);
    let other_class: id = msg![env; other class];
    let is_uuid: bool = msg![env; other_class isSubclassOfClass:b_uuid_class];
    if !is_uuid { return 1; }
    let b = env.objc.borrow::<NSUUIDHostObject>(other).bytes.to_string();
    match a.cmp(&b) {
        std::cmp::Ordering::Less    => -1,
        std::cmp::Ordering::Equal   =>  0,
        std::cmp::Ordering::Greater =>  1,
    }
}

// =========================================================================
// MARK: - NSCopying
// =========================================================================

- (id)copyWithZone:(NSZonePtr)_zone {
    // UUIDs are immutable — return self retained.
    retain(env, this);
    this
}

// =========================================================================
// MARK: - NSCoding
// =========================================================================

- (())encodeWithCoder:(id)coder {
    // Encode the UUID string under the key "NS.uuidbytes".
    let key = ns_string::get_static_str(env, "NS.uuidbytes");
    let uuid_str: id = msg![env; this UUIDString];
    let _: () = msg![env; coder encodeObject:uuid_str forKey:key];
}

- (id)initWithCoder:(id)coder {
    let key = ns_string::get_static_str(env, "NS.uuidbytes");
    let uuid_str: id = msg![env; coder decodeObjectForKey:key];
    if uuid_str != nil {
        msg![env; this initWithUUIDString:uuid_str]
    } else {
        // Fall back to raw bytes if stored that way.
        let bytes_key = ns_string::get_static_str(env, "NS.bytes");
        let data: id = msg![env; coder decodeObjectForKey:bytes_key];
        if data != nil {
            let ptr: crate::mem::ConstPtr<u8> = msg![env; data bytes];
            msg![env; this initWithUUIDBytes:ptr]
        } else {
            msg![env; this init]
        }
    }
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    msg![env; this UUIDString]
}

- (id)debugDescription {
    let uuid_str: id = msg![env; this UUIDString];
    let s = format!(
        "<NSUUID: {:?}; {}>",
        this,
        ns_string::to_rust_string(env, uuid_str)
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};

// =========================================================================
// MARK: - Public Rust helper
// =========================================================================

/// Create a new autoreleased `NSUUID*` with a random UUID.
pub fn new_uuid(env: &mut crate::Environment) -> id {
    msg_class![env; NSUUID UUID]
}

/// Create an `NSUUID*` from a Rust string, returning nil on parse failure.
pub fn uuid_from_string(env: &mut crate::Environment, s: &str) -> id {
    let ns = ns_string::from_rust_string(env, s.to_string());
    let ns = crate::objc::autorelease(env, ns);
    let alloc: id = msg_class![env; NSUUID alloc];
    msg![env; alloc initWithUUIDString:ns]
}
