/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `<uuid/uuid.h>` — Apple's libuuid C API (shipped as part of libSystem).
//!
//! Apple references:
//! * `uuid(3)` man page:
//!   <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/uuid.3.html>
//! * Header `<uuid/uuid.h>` (open-source `libutil` /
//!   `Libsystem` projects).
//!
//! ```c
//! typedef unsigned char uuid_t[16];
//! typedef char uuid_string_t[37];
//!
//! void  uuid_clear(uuid_t out);
//! void  uuid_copy(uuid_t dst, const uuid_t src);
//! int   uuid_compare(const uuid_t a, const uuid_t b);
//! int   uuid_is_null(const uuid_t uu);
//! void  uuid_generate_random(uuid_t out);
//! void  uuid_generate_time(uuid_t out);
//! void  uuid_generate(uuid_t out);            /* prefers random */
//! int   uuid_parse(const char *in, uuid_t out);
//! void  uuid_unparse(const uuid_t uu, char *out);
//! void  uuid_unparse_lower(const uuid_t uu, char *out);
//! void  uuid_unparse_upper(const uuid_t uu, char *out);
//! ```
//!
//! These are pure data manipulators — no syscalls, no allocations, no
//! locale dependence — so we can implement them entirely in host code
//! and round-trip the buffers through the guest's memory.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_uuid::UuidBytes;
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;

/// `void uuid_clear(uuid_t out)` — fill the 16-byte buffer with zeros.
fn uuid_clear(env: &mut Environment, out: MutPtr<u8>) {
    if out.is_null() {
        return;
    }
    for i in 0..16 {
        env.mem.write(out + i, 0u8);
    }
}

/// `void uuid_copy(uuid_t dst, const uuid_t src)`.
fn uuid_copy(env: &mut Environment, dst: MutPtr<u8>, src: ConstPtr<u8>) {
    if dst.is_null() || src.is_null() {
        return;
    }
    for i in 0..16 {
        let b: u8 = env.mem.read(src + i);
        env.mem.write(dst + i, b);
    }
}

/// `int uuid_compare(const uuid_t a, const uuid_t b)` — lexicographic
/// comparison returning a negative, zero, or positive value as per the
/// usual `memcmp` contract.
fn uuid_compare(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>) -> i32 {
    if a == b {
        return 0;
    }
    if a.is_null() {
        return -1;
    }
    if b.is_null() {
        return 1;
    }
    for i in 0..16 {
        let ai: u8 = env.mem.read(a + i);
        let bi: u8 = env.mem.read(b + i);
        match ai.cmp(&bi) {
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Greater => return 1,
            std::cmp::Ordering::Equal => continue,
        }
    }
    0
}

/// `int uuid_is_null(const uuid_t uu)` — non-zero if all 16 bytes are
/// zero, zero otherwise.
fn uuid_is_null(env: &mut Environment, uu: ConstPtr<u8>) -> i32 {
    if uu.is_null() {
        return 1;
    }
    for i in 0..16 {
        let b: u8 = env.mem.read(uu + i);
        if b != 0 {
            return 0;
        }
    }
    1
}

fn write_bytes(env: &mut Environment, out: MutPtr<u8>, bytes: &[u8; 16]) {
    for (i, b) in bytes.iter().enumerate() {
        env.mem.write(out + i as u32, *b);
    }
}

/// `void uuid_generate_random(uuid_t out)` — RFC 4122 version-4 UUID.
fn uuid_generate_random(env: &mut Environment, out: MutPtr<u8>) {
    if out.is_null() {
        return;
    }
    let UuidBytes(bytes) = UuidBytes::random();
    write_bytes(env, out, &bytes);
}

/// `void uuid_generate_time(uuid_t out)` — RFC 4122 version-1 UUID
/// (timestamp + MAC). touchHLE has no MAC address, so we fall back to
/// the random implementation. The output is still a syntactically
/// valid UUID with the version-4 bits set.
fn uuid_generate_time(env: &mut Environment, out: MutPtr<u8>) {
    uuid_generate_random(env, out);
}

/// `void uuid_generate(uuid_t out)` — Apple's documentation says this
/// prefers random when a high-quality entropy source is available.
fn uuid_generate(env: &mut Environment, out: MutPtr<u8>) {
    uuid_generate_random(env, out);
}

fn read_bytes(env: &mut Environment, uu: ConstPtr<u8>) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for i in 0..16 {
        bytes[i] = env.mem.read(uu + i as u32);
    }
    bytes
}

fn write_string(env: &mut Environment, out: MutPtr<u8>, s: &str) {
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        env.mem.write(out + i as u32, *b);
    }
    env.mem.write(out + bytes.len() as u32, 0u8);
}

/// `int uuid_parse(const char *in, uuid_t out)` — returns 0 on success,
/// -1 if the string is malformed (per Apple's man page).
fn uuid_parse(env: &mut Environment, input: ConstPtr<u8>, out: MutPtr<u8>) -> i32 {
    if input.is_null() || out.is_null() {
        return -1;
    }
    // Read a NUL-terminated C string of bounded length (max ~64 bytes
    // for a UUID with braces).
    let mut buf = Vec::with_capacity(64);
    for i in 0..64u32 {
        let b: u8 = env.mem.read(input + i);
        if b == 0 {
            break;
        }
        buf.push(b);
    }
    let s = match std::str::from_utf8(&buf) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let Some(UuidBytes(bytes)) = UuidBytes::from_string(s) else {
        return -1;
    };
    write_bytes(env, out, &bytes);
    0
}

/// `void uuid_unparse_upper(const uuid_t uu, char *out)` — uppercase
/// `XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX\0`. The output buffer must be
/// at least `UUID_STRING_LENGTH` (37) bytes.
fn uuid_unparse_upper(env: &mut Environment, uu: ConstPtr<u8>, out: MutPtr<u8>) {
    if uu.is_null() || out.is_null() {
        return;
    }
    let bytes = read_bytes(env, uu);
    write_string(env, out, &UuidBytes(bytes).to_string());
}

/// `void uuid_unparse_lower(const uuid_t uu, char *out)`.
fn uuid_unparse_lower(env: &mut Environment, uu: ConstPtr<u8>, out: MutPtr<u8>) {
    if uu.is_null() || out.is_null() {
        return;
    }
    let bytes = read_bytes(env, uu);
    write_string(env, out, &UuidBytes(bytes).to_string_lowercase());
}

/// `void uuid_unparse(const uuid_t uu, char *out)` — Apple's libuuid
/// historically defaults to uppercase, matching `uuid_unparse_upper`.
fn uuid_unparse(env: &mut Environment, uu: ConstPtr<u8>, out: MutPtr<u8>) {
    uuid_unparse_upper(env, uu, out);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(uuid_clear(_)),
    export_c_func!(uuid_copy(_, _)),
    export_c_func!(uuid_compare(_, _)),
    export_c_func!(uuid_is_null(_)),
    export_c_func!(uuid_generate(_)),
    export_c_func!(uuid_generate_random(_)),
    export_c_func!(uuid_generate_time(_)),
    export_c_func!(uuid_parse(_, _)),
    export_c_func!(uuid_unparse(_, _)),
    export_c_func!(uuid_unparse_lower(_, _)),
    export_c_func!(uuid_unparse_upper(_, _)),
];
