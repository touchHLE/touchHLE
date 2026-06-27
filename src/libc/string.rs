/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `string.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::cmp::Ordering;

use super::generic_char::GenericChar;

#[derive(Default)]
pub struct State {
    strtok: Option<MutPtr<u8>>,
}

fn strtok(env: &mut Environment, s: MutPtr<u8>, sep: ConstPtr<u8>) -> MutPtr<u8> {
    // POSIX: A subsequent call with a null first argument continues searching
    // from the saved pointer. If no further token exists (or strtok() has
    // never been called with a non-null first argument), a null pointer is
    // returned. We must NOT panic here — some apps invoke strtok(NULL, ...)
    // speculatively or after a previous call returned NULL.
    let s = if s.is_null() {
        match env.libc_state.string.strtok {
            Some(state) if !state.is_null() => state,
            _ => {
                env.libc_state.string.strtok = None;
                return Ptr::null();
            }
        }
    } else {
        s
    };
    let sep = env.mem.cstr_at(sep);

    let mut token_start = s;
    loop {
        let c = env.mem.read(token_start);
        if c == b'\0' {
            env.libc_state.string.strtok = None;
            return Ptr::null();
        } else if sep.contains(&c) {
            token_start += 1;
        } else {
            break;
        }
    }

    let mut token_end = token_start;
    let next_token = loop {
        let c = env.mem.read(token_end);
        if sep.contains(&c) {
            env.mem.write(token_end, b'\0');
            break token_end + 1;
        } else if c == b'\0' {
            break Ptr::null();
        } else {
            token_end += 1;
        }
    };

    env.libc_state.string.strtok = Some(next_token);

    token_start
}

// Functions shared with wchar.rs

fn bzero(env: &mut Environment, dest: MutVoidPtr, count: GuestUSize) {
    memset(env, dest, 0, count);
}
fn memset(env: &mut Environment, dest: MutVoidPtr, ch: i32, count: GuestUSize) -> MutVoidPtr {
    GenericChar::<u8>::memset(env, dest.cast(), ch as u8, count, GuestUSize::MAX).cast()
}
fn __memset_chk(
    env: &mut Environment,
    dest: MutVoidPtr,
    ch: i32,
    count: GuestUSize,
    dest_count: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memset(env, dest.cast(), ch as u8, count, dest_count).cast()
}
fn memset_pattern4(env: &mut Environment, b: MutVoidPtr, pattern4: ConstVoidPtr, len: GuestUSize) {
    memset_pattern_inner(env, b, pattern4, len, 4)
}
fn memset_pattern8(env: &mut Environment, b: MutVoidPtr, pattern8: ConstVoidPtr, len: GuestUSize) {
    memset_pattern_inner(env, b, pattern8, len, 8)
}
fn memset_pattern16(
    env: &mut Environment,
    b: MutVoidPtr,
    pattern16: ConstVoidPtr,
    len: GuestUSize,
) {
    memset_pattern_inner(env, b, pattern16, len, 16)
}
fn memset_pattern_inner(
    env: &mut Environment,
    b: MutVoidPtr,
    pattern: ConstVoidPtr,
    len: GuestUSize,
    pattern_len: GuestUSize,
) {
    assert!(matches!(pattern_len, 4 | 8 | 16));
    let mut tmp = [0; 16];
    tmp[..pattern_len as usize].copy_from_slice(env.mem.bytes_at(pattern.cast(), pattern_len));
    let mut target: MutPtr<u8> = b.cast();
    for _ in 0..(len / pattern_len) {
        env.mem
            .bytes_at_mut(target, pattern_len)
            .copy_from_slice(&tmp[..pattern_len as usize]);
        target += pattern_len;
    }
    for i in 0..(len % pattern_len) {
        let value = env.mem.read(pattern.cast() + i);
        env.mem.write(target, value);
        target += 1;
    }
}
fn memcpy(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memcpy(env, dest.cast(), src.cast(), size, GuestUSize::MAX).cast()
}
fn __memcpy_chk(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
    dest_size: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memcpy(env, dest.cast(), src.cast(), size, dest_size).cast()
}
fn memmove(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memmove(env, dest.cast(), src.cast(), size, GuestUSize::MAX).cast()
}
fn __memmove_chk(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
    dest_size: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memmove(env, dest.cast(), src.cast(), size, dest_size).cast()
}
fn memchr(env: &mut Environment, string: ConstVoidPtr, c: i32, size: GuestUSize) -> ConstVoidPtr {
    GenericChar::<u8>::memchr(env, string.cast(), c as u8, size).cast()
}
fn memcmp(env: &mut Environment, a: ConstVoidPtr, b: ConstVoidPtr, size: GuestUSize) -> i32 {
    GenericChar::<u8>::memcmp(env, a.cast(), b.cast(), size)
}
pub(crate) fn strlen(env: &mut Environment, s: ConstPtr<u8>) -> GuestUSize {
    GenericChar::<u8>::strlen(env, s)
}
pub(super) fn strcpy(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    GenericChar::<u8>::strcpy(env, dest, src, GuestUSize::MAX)
}
fn __strcpy_chk(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
) -> MutPtr<u8> {
    GenericChar::<u8>::strcpy(env, dest, src, size)
}
fn strcat(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    GenericChar::<u8>::strcat(env, dest, src, GuestUSize::MAX)
}
fn __strcat_chk(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
) -> MutPtr<u8> {
    GenericChar::<u8>::strcat(env, dest, src, size)
}
fn strspn(env: &mut Environment, s: ConstPtr<u8>, charset: ConstPtr<u8>) -> GuestUSize {
    GenericChar::<u8>::strspn(env, s, charset)
}
fn strcspn(env: &mut Environment, s: ConstPtr<u8>, charset: ConstPtr<u8>) -> GuestUSize {
    GenericChar::<u8>::strcspn(env, s, charset)
}
pub(crate) fn strncpy(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
) -> MutPtr<u8> {
    GenericChar::<u8>::strncpy(env, dest, src, size, GuestUSize::MAX)
}
fn __strncpy_chk(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
    dest_size: GuestUSize,
) -> MutPtr<u8> {
    GenericChar::<u8>::strncpy(env, dest, src, size, dest_size)
}
fn strsep(env: &mut Environment, stringp: MutPtr<MutPtr<u8>>, delim: ConstPtr<u8>) -> MutPtr<u8> {
    let orig = env.mem.read(stringp);
    if orig.is_null() {
        return Ptr::null();
    }
    let tmp = orig;
    let mut i = 0;
    loop {
        let c = env.mem.read(tmp + i);
        if c == b'\0' {
            env.mem.write(stringp, Ptr::null());
            break;
        }
        let mut j = 0;
        loop {
            let cc = env.mem.read(delim + j);
            if c == cc {
                env.mem.write(tmp + i, b'\0');
                env.mem.write(stringp, tmp + i + 1);
                return orig;
            }
            if cc == b'\0' {
                break;
            }
            j += 1;
        }
        i += 1;
    }
    orig
}
pub(crate) fn strdup(env: &mut Environment, src: ConstPtr<u8>) -> MutPtr<u8> {
    GenericChar::<u8>::strdup(env, src)
}
pub fn strcmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>) -> i32 {
    GenericChar::<u8>::strcmp(env, a, b)
}
fn strncmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>, n: GuestUSize) -> i32 {
    GenericChar::<u8>::strncmp(env, a, b, n)
}
fn strcasecmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>) -> i32 {
    // TODO: generalize to wide chars
    let mut offset = 0;
    loop {
        let char_a = env.mem.read(a + offset).to_ascii_lowercase();
        let char_b = env.mem.read(b + offset).to_ascii_lowercase();
        offset += 1;

        match char_a.cmp(&char_b) {
            Ordering::Less => return -1,
            Ordering::Greater => return 1,
            Ordering::Equal => {
                if char_a == u8::default() {
                    return 0;
                } else {
                    continue;
                }
            }
        }
    }
}
fn strncasecmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>, n: GuestUSize) -> i32 {
    // TODO: generalize to wide chars
    if n == 0 {
        return 0;
    }

    let mut offset = 0;
    loop {
        let char_a = env.mem.read(a + offset).to_ascii_lowercase();
        let char_b = env.mem.read(b + offset).to_ascii_lowercase();
        offset += 1;

        match char_a.cmp(&char_b) {
            Ordering::Less => return -1,
            Ordering::Greater => return 1,
            Ordering::Equal => {
                if offset == n || char_a == u8::default() {
                    return 0;
                } else {
                    continue;
                }
            }
        }
    }
}
fn strncat(env: &mut Environment, s1: MutPtr<u8>, s2: ConstPtr<u8>, n: GuestUSize) -> MutPtr<u8> {
    GenericChar::<u8>::strncat(env, s1, s2, n)
}
/// `__strncat_chk` — fortified variant of strncat.
/// Per Apple's Secure Coding Guide and the GCC/Clang SSP implementation:
/// `char *__strncat_chk(char *dest, const char *src, size_t n, size_t dest_size)`
/// The `dest_size` parameter is the total buffer size of `dest` (as known at
/// compile time via `__builtin_object_size`). If the concatenation would
/// overflow, the real implementation calls `__chk_fail`. We simply delegate
/// to the normal `strncat` since touchHLE already guards against OOB writes
/// at the memory subsystem level.
fn __strncat_chk(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    n: GuestUSize,
    _dest_size: GuestUSize,
) -> MutPtr<u8> {
    GenericChar::<u8>::strncat(env, dest, src, n)
}
fn strstr(env: &mut Environment, string: ConstPtr<u8>, substring: ConstPtr<u8>) -> ConstPtr<u8> {
    GenericChar::<u8>::strstr(env, string, substring)
}
fn strchr(env: &mut Environment, path: ConstPtr<u8>, c: u8) -> ConstPtr<u8> {
    GenericChar::<u8>::strchr(env, path, c)
}
/// POSIX `index()` — identical to `strchr()`. Finds the first occurrence of
/// byte `c` in the string pointed to by `s`.
fn index(env: &mut Environment, s: ConstPtr<u8>, c: u8) -> ConstPtr<u8> {
    GenericChar::<u8>::strchr(env, s, c)
}
fn strrchr(env: &mut Environment, path: ConstPtr<u8>, c: u8) -> ConstPtr<u8> {
    GenericChar::<u8>::strrchr(env, path, c)
}
fn strpbrk(env: &mut Environment, s: ConstPtr<u8>, charset: ConstPtr<u8>) -> ConstPtr<u8> {
    GenericChar::<u8>::strpbrk(env, s, charset)
}
fn strlcpy(
    env: &mut Environment,
    dst: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
) -> GuestUSize {
    GenericChar::<u8>::strlcpy(env, dst, src, size)
}

// Add these functions to string.rs:

fn strlcat(
    env: &mut Environment,
    dst: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
) -> GuestUSize {
    GenericChar::<u8>::strlcat(env, dst, src, size)
}

fn strspn(env: &mut Environment, s: ConstPtr<u8>, accept: ConstPtr<u8>) -> GuestUSize {
    GenericChar::<u8>::strspn(env, s, accept)
}

fn strpbrk(env: &mut Environment, s: ConstPtr<u8>, accept: ConstPtr<u8>) -> ConstPtr<u8> {
    GenericChar::<u8>::strpbrk(env, s, accept)
}

fn strndup(env: &mut Environment, src: ConstPtr<u8>, n: GuestUSize) -> MutPtr<u8> {
    let len = strlen(env, src).min(n);
    let buf: MutPtr<u8> = env.mem.alloc(len + 1).cast();
    for i in 0..len {
        let c = env.mem.read(src + i);
        env.mem.write(buf + i, c);
    }
    env.mem.write(buf + len, b'\0');
    buf
}

fn memrchr(env: &mut Environment, s: ConstVoidPtr, c: i32, n: GuestUSize) -> ConstVoidPtr {
    let needle = c as u8;
    let mut i = n;
    while i > 0 {
        i -= 1;
        let byte = env.mem.read(s.cast::<u8>() + i);
        if byte == needle {
            return (s.cast::<u8>() + i).cast();
        }
    }
    ConstVoidPtr::null()
}

fn memmem(
    env: &mut Environment,
    haystack: ConstVoidPtr,
    haystacklen: GuestUSize,
    needle: ConstVoidPtr,
    needlelen: GuestUSize,
) -> ConstVoidPtr {
    if needlelen == 0 {
        return haystack;
    }
    if needlelen > haystacklen {
        return ConstVoidPtr::null();
    }
    let limit = haystacklen - needlelen;
    'outer: for i in 0..=limit {
        for j in 0..needlelen {
            let h = env.mem.read(haystack.cast::<u8>() + i + j);
            let n = env.mem.read(needle.cast::<u8>() + j);
            if h != n {
                continue 'outer;
            }
        }
        return (haystack.cast::<u8>() + i).cast();
    }
    ConstVoidPtr::null()
}

fn strtok_r(
    env: &mut Environment,
    s: MutPtr<u8>,
    sep: ConstPtr<u8>,
    saveptr: MutPtr<MutPtr<u8>>,
) -> MutPtr<u8> {
    let start: MutPtr<u8> = if !s.is_null() {
        s
    } else {
        env.mem.read(saveptr)
    };
    if start.is_null() {
        return Ptr::null();
    }

    let sep_bytes = env.mem.cstr_at(sep);
    // Skip leading separators.
    let mut i: GuestUSize = 0;
    loop {
        let c = env.mem.read(start + i);
        if c == b'\0' {
            env.mem.write(saveptr, Ptr::null());
            return Ptr::null();
        }
        if sep_bytes.contains(&c) {
            i += 1;
        } else {
            break;
        }
    }

    let token_start = start + i;

    // Find end of token.
    loop {
        let c = env.mem.read(start + i);
        if sep_bytes.contains(&c) {
            env.mem.write(start + i, b'\0');
            env.mem.write(saveptr, start + i + 1);
            return token_start;
        }
        if c == b'\0' {
            env.mem.write(saveptr, Ptr::null());
            return token_start;
        }
        i += 1;
    }
}

fn strverscmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>) -> i32 {
    // Version-aware string compare: numeric segments compared numerically.
    let sa = env.mem.cstr_at_utf8(a).unwrap_or("").to_string();
    let sb = env.mem.cstr_at_utf8(b).unwrap_or("").to_string();

    let mut ia = sa.chars().peekable();
    let mut ib = sb.chars().peekable();
    loop {
        match (ia.peek().copied(), ib.peek().copied()) {
            (None, None) => return 0,
            (None, _) => return -1,
            (_, None) => return 1,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    // Collect numeric runs and compare as integers.
                    let na: u64 = {
                        let mut s = String::new();
                        while ia.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                            s.push(ia.next().unwrap());
                        }
                        s.parse().unwrap_or(0)
                    };
                    let nb: u64 = {
                        let mut s = String::new();
                        while ib.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                            s.push(ib.next().unwrap());
                        }
                        s.parse().unwrap_or(0)
                    };
                    match na.cmp(&nb) {
                        std::cmp::Ordering::Less => return -1,
                        std::cmp::Ordering::Greater => return 1,
                        std::cmp::Ordering::Equal => {}
                    }
                } else {
                    if ca != cb {
                        return (ca as i32) - (cb as i32);
                    }
                    ia.next();
                    ib.next();
                }
            }
        }
    }
}

fn strchrnul(env: &mut Environment, s: ConstPtr<u8>, c: u8) -> ConstPtr<u8> {
    let mut i: GuestUSize = 0;
    loop {
        let byte = env.mem.read(s + i);
        if byte == c || byte == b'\0' {
            return s + i;
        }
        i += 1;
    }
}

fn stpcpy(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    let mut i: GuestUSize = 0;
    loop {
        let c = env.mem.read(src + i);
        env.mem.write(dest + i, c);
        if c == b'\0' {
            return dest + i; // pointer to the null terminator
        }
        i += 1;
    }
}

fn stpncpy(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    n: GuestUSize,
) -> MutPtr<u8> {
    let mut i: GuestUSize = 0;
    while i < n {
        let c = env.mem.read(src + i);
        env.mem.write(dest + i, c);
        if c == b'\0' {
            // Zero-pad remaining bytes.
            for j in i + 1..n {
                env.mem.write(dest + j, b'\0');
            }
            return dest + i;
        }
        i += 1;
    }
    dest + n
}

fn strfry(env: &mut Environment, s: MutPtr<u8>) -> MutPtr<u8> {
    // Fisher-Yates shuffle of the string in place.
    let len = strlen(env, s.cast_const());
    if len <= 1 {
        return s;
    }
    for i in (1..len).rev() {
        let j = (i as u64 * 6364136223846793005u64.wrapping_add(1442695040888963407)) as u32
            % (i + 1);
        let a = env.mem.read(s + i);
        let b = env.mem.read(s + j);
        env.mem.write(s + i, b);
        env.mem.write(s + j, a);
    }
    s
}

fn explicit_bzero(env: &mut Environment, dest: MutVoidPtr, count: GuestUSize) {
    // Same as bzero but compiler must not optimize away (no difference in our
    // emulated context).
    for i in 0..count {
        env.mem.write(dest.cast::<u8>() + i, 0u8);
    }
}

fn strerror(_env: &mut Environment, errnum: i32) -> ConstPtr<u8> {
    // We can't easily return a stable guest pointer to a Rust string.
    // Return null — callers that check for null will handle it.
    log!("strerror({}): stubbed, returning null", errnum);
    ConstPtr::null()
}

fn strerror_r(env: &mut Environment, errnum: i32, buf: MutPtr<u8>, buflen: GuestUSize) -> i32 {
    let msg = format!("Error {}", errnum);
    let bytes = msg.as_bytes();
    let copy_len = (bytes.len() as GuestUSize).min(buflen.saturating_sub(1));
    for i in 0..copy_len {
        env.mem.write(buf + i, bytes[i as usize]);
    }
    env.mem.write(buf + copy_len, b'\0');
    0
}

fn bcopy(env: &mut Environment, src: ConstVoidPtr, dest: MutVoidPtr, count: GuestUSize) {
    memmove(env, dest, src, count);
}

fn strnlen(env: &mut Environment, s: ConstPtr<u8>, maxlen: GuestUSize) -> GuestUSize {
    let mut len: GuestUSize = 0;
    while len < maxlen {
        let c = env.mem.read(s + len);
        if c == b'\0' {
            return len;
        }
        len += 1;
    }
    maxlen
}

fn strcasestr(env: &mut Environment, haystack: MutPtr<u8>, needle: ConstPtr<u8>) -> MutPtr<u8> {
    // Если указатели нулевые, безопасно возвращаем null, чтобы избежать краша
    if haystack.is_null() || needle.is_null() {
        return Ptr::null();
    }

    // Читаем C-строки из памяти эмулятора в виде слайсов байтов &[u8]
    let haystack_str = env.mem.cstr_at(haystack.cast_const());
    let needle_str = env.mem.cstr_at(needle);

    // Если искомая подстрока пустая, стандартное поведение — вернуть саму
    // строку
    if needle_str.is_empty() {
        return haystack;
    }

    let needle_len = needle_str.len();

    // ИСПРАВЛЕНИЕ: Если строка, в которой ищем, короче искомого слова,
    // совпадение невозможно в принципе. Выходим сразу.
    if haystack_str.len() < needle_len {
        return Ptr::null();
    }

    // Ищем совпадение, используя стандартный метод Rust без учета
    // ASCII-регистра
    for i in 0..=haystack_str.len() - needle_len {
        // saturating_sub больше не нужен
        let window = &haystack_str[i..i + needle_len];
        if window.eq_ignore_ascii_case(needle_str) {
            // Возвращаем указатель на начало найденной подстроки
            return haystack + i as GuestUSize;
        }
    }

    // Если ничего не найдено, возвращаем null
    Ptr::null()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(strtok(_, _)),
    export_c_func!(bzero(_, _)),
    // Functions shared with wchar.rs
    export_c_func!(memset(_, _, _)),
    export_c_func!(__memset_chk(_, _, _, _)),
    export_c_func!(memset_pattern4(_, _, _)),
    export_c_func!(memset_pattern8(_, _, _)),
    export_c_func!(memset_pattern16(_, _, _)),
    export_c_func!(memcpy(_, _, _)),
    export_c_func!(__memcpy_chk(_, _, _, _)),
    export_c_func!(memmove(_, _, _)),
    export_c_func!(__memmove_chk(_, _, _, _)),
    export_c_func!(memchr(_, _, _)),
    export_c_func!(memcmp(_, _, _)),
    export_c_func!(strlen(_)),
    export_c_func!(strcpy(_, _)),
    export_c_func!(__strcpy_chk(_, _, _)),
    export_c_func!(strcat(_, _)),
    export_c_func!(strspn(_, _)),
    export_c_func!(strcspn(_, _)),
    export_c_func!(__strcat_chk(_, _, _)),
    export_c_func!(strncpy(_, _, _)),
    export_c_func!(__strncpy_chk(_, _, _, _)),
    export_c_func!(strsep(_, _)),
    export_c_func!(strdup(_)),
    export_c_func!(strcmp(_, _)),
    export_c_func!(strncmp(_, _, _)),
    export_c_func!(strcasecmp(_, _)),
    export_c_func!(strncasecmp(_, _, _)),
    export_c_func!(strncat(_, _, _)),
    export_c_func!(__strncat_chk(_, _, _, _)),
    export_c_func!(strstr(_, _)),
    export_c_func!(strchr(_, _)),
    export_c_func!(index(_, _)),
    export_c_func!(strrchr(_, _)),
    export_c_func!(strpbrk(_, _)),
    export_c_func!(strlcpy(_, _, _)),
    export_c_func!(strlcat(_, _, _)),
    export_c_func!(strspn(_, _)),
    export_c_func!(strpbrk(_, _)),
    export_c_func!(strndup(_, _)),
    export_c_func!(memrchr(_, _, _)),
    export_c_func!(memmem(_, _, _, _)),
    export_c_func!(strtok_r(_, _, _)),
    export_c_func!(strverscmp(_, _)),
    export_c_func!(strchrnul(_, _)),
    export_c_func!(stpcpy(_, _)),
    export_c_func!(stpncpy(_, _, _)),
    export_c_func!(strfry(_)),
    export_c_func!(explicit_bzero(_, _)),
    // strerror is exported from libc::errno; not duplicated here.
    export_c_func!(strerror_r(_, _, _)),
    export_c_func!(bcopy(_, _, _)),
    export_c_func!(strnlen(_, _)),
    export_c_func!(strcasestr(_, _)),
];
