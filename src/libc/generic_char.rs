/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Generic versions of string functions for use by [super::string] and
//! [super::wchar].

use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, Ptr, SafeRead};
use crate::Environment;
use std::cmp::Ordering;
use std::fmt::Debug;

/// This type is never actually constructed, it just enables us to move all the
/// bounds on `T` to the `impl` block.
pub(super) struct GenericChar<T> {
    _spooky: std::marker::PhantomData<T>,
}

impl<T: Copy + Default + Eq + Ord + SafeRead + Debug> GenericChar<T> {
    fn null() -> T {
        Default::default()
    }

    pub(super) fn memset(
        env: &mut Environment,
        dest: MutPtr<T>,
        ch: T,
        count: GuestUSize,
        dest_count: GuestUSize,
    ) -> MutPtr<T> {
        // FORTIFY_SOURCE-style overflow check. On real Apple libc the guest
        // would abort via __chk_fail(); we cap the write at the declared
        // buffer size, log loudly, and let the guest keep running. A host
        // panic would also stop the emulator, which is worse than a possibly
        // truncated memset.
        let actual = if count > dest_count {
            log!(
                "Warning: __memset_chk: count {} > dest_count {}; truncating instead of aborting.",
                count,
                dest_count
            );
            dest_count
        } else {
            count
        };
        for i in 0..actual {
            env.mem.write(dest + i, ch);
        }
        dest
    }

    pub(super) fn memcpy(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
        dest_size: GuestUSize,
    ) -> MutPtr<T> {
        let actual = if size > dest_size {
            log!(
                "Warning: __memcpy_chk: size {} > dest_size {}; truncating instead of aborting.",
                size,
                dest_size
            );
            dest_size
        } else {
            size
        };
        env.mem
            .memmove(dest.cast(), src.cast(), actual * guest_size_of::<T>());
        dest
    }

    pub(super) fn memmove(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
        dest_size: GuestUSize,
    ) -> MutPtr<T> {
        let actual = if size > dest_size {
            log!(
                "Warning: __memmove_chk: size {} > dest_size {}; truncating instead of aborting.",
                size,
                dest_size
            );
            dest_size
        } else {
            size
        };
        env.mem
            .memmove(dest.cast(), src.cast(), actual * guest_size_of::<T>());
        dest
    }

    pub(super) fn memcmp(
        env: &mut Environment,
        a: ConstPtr<T>,
        b: ConstPtr<T>,
        n: GuestUSize,
    ) -> i32 {
        let mut offset = 0;
        while offset < n {
            let char_a = env.mem.read(a + offset);
            let char_b = env.mem.read(b + offset);
            offset += 1;

            match char_a.cmp(&char_b) {
                Ordering::Less => return -1,
                Ordering::Greater => return 1,
                Ordering::Equal => continue,
            }
        }
        0
    }

    pub(super) fn memchr(
        env: &mut Environment,
        string: ConstPtr<T>,
        c: T,
        size: GuestUSize,
    ) -> ConstPtr<T> {
        for i in 0..size {
            if env.mem.read(string + i) == c {
                return string + i;
            }
        }
        Ptr::null()
    }

    pub(super) fn strlen(env: &mut Environment, s: ConstPtr<T>) -> GuestUSize {
        let mut i = 0;
        while env.mem.read(s + i) != Self::null() {
            i += 1;
        }
        i
    }

    pub(super) fn strcpy(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        mut bufsz: GuestUSize,
    ) -> MutPtr<T> {
        {
            let (mut dest_p, mut src) = (dest, src);
            loop {
                if bufsz == 0 {
                    // Buffer is full and we have not seen NUL yet. Write a
                    // NUL into the last byte (defensive — keeps the string
                    // C-terminated even though the caller asked for an
                    // unbounded copy) and bail out instead of panicking the
                    // host.
                    log!(
                        "Warning: __strcpy_chk-style copy hit buffer end at {:?} without seeing NUL; truncating.",
                        dest_p
                    );
                    env.mem.write(dest_p, Self::null());
                    break;
                }
                let c = env.mem.read(src);
                env.mem.write(dest_p, c);
                if c == Self::null() {
                    break;
                }
                dest_p += 1;
                src += 1;
                bufsz -= 1;
            }
        }
        dest
    }

    pub(super) fn strcat(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        bufsz: GuestUSize,
    ) -> MutPtr<T> {
        {
            let dest_len = Self::strlen(env, dest.cast_const());
            let tail = dest + dest_len;
            let remaining = match bufsz.checked_sub(dest_len) {
                Some(r) => r,
                None => {
                    log!(
                        "Warning: __strcat_chk overflow: dest_len {} > bufsz {}; refusing to append.",
                        dest_len,
                        bufsz
                    );
                    return dest;
                }
            };
            Self::strcpy(env, tail, src, remaining);
        }
        dest
    }

    pub(super) fn strspn(
        env: &mut Environment,
        s: ConstPtr<T>,
        charset: ConstPtr<T>,
    ) -> GuestUSize {
        let mut i = 0;
        loop {
            let c = env.mem.read(s + i);
            if c == Self::null() {
                break;
            }
            let mut j = 0;
            loop {
                let cc = env.mem.read(charset + j);
                if c == cc {
                    break;
                }
                if cc == Self::null() {
                    return i;
                }
                j += 1;
            }
            i += 1;
        }
        i
    }

    pub(super) fn strcspn(
        env: &mut Environment,
        s: ConstPtr<T>,
        charset: ConstPtr<T>,
    ) -> GuestUSize {
        let mut i = 0;
        loop {
            let c = env.mem.read(s + i);
            if c == Self::null() {
                break;
            }
            let mut j = 0;
            loop {
                let cc = env.mem.read(charset + j);
                if c == cc {
                    return i;
                }
                if cc == Self::null() {
                    break;
                }
                j += 1;
            }
            i += 1;
        }
        i
    }

    pub(super) fn strncpy(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
        dest_size: GuestUSize,
    ) -> MutPtr<T> {
        let actual = if dest_size < size {
            log!(
                "Warning: __strncpy_chk: size {} > dest_size {}; truncating instead of aborting.",
                size,
                dest_size
            );
            dest_size
        } else {
            size
        };
        let mut end = false;
        for i in 0..actual {
            if !end {
                let c = env.mem.read(src + i);
                if c == Self::null() {
                    end = true;
                }
                env.mem.write(dest + i, c);
            } else {
                env.mem.write(dest + i, Self::null());
            }
        }
        dest
    }

    pub(super) fn strdup(env: &mut Environment, src: ConstPtr<T>) -> MutPtr<T> {
        let len = Self::strlen(env, src);
        let new = env.mem.alloc((len + 1) * guest_size_of::<T>()).cast();
        Self::strcpy(env, new, src, GuestUSize::MAX)
    }

    pub(super) fn strcmp(env: &mut Environment, a: ConstPtr<T>, b: ConstPtr<T>) -> i32 {
        let mut offset = 0;
        loop {
            let char_a = env.mem.read(a + offset);
            let char_b = env.mem.read(b + offset);
            offset += 1;

            match char_a.cmp(&char_b) {
                Ordering::Less => return -1,
                Ordering::Greater => return 1,
                Ordering::Equal => {
                    if char_a == Self::null() {
                        return 0;
                    } else {
                        continue;
                    }
                }
            }
        }
    }

    pub(super) fn strncmp(
        env: &mut Environment,
        a: ConstPtr<T>,
        b: ConstPtr<T>,
        n: GuestUSize,
    ) -> i32 {
        if n == 0 {
            return 0;
        }

        let mut offset = 0;
        loop {
            let char_a = env.mem.read(a + offset);
            let char_b = env.mem.read(b + offset);
            offset += 1;

            match char_a.cmp(&char_b) {
                Ordering::Less => return -1,
                Ordering::Greater => return 1,
                Ordering::Equal => {
                    if offset == n || char_a == Self::null() {
                        return 0;
                    } else {
                        continue;
                    }
                }
            }
        }
    }

    pub(super) fn strncat(
        env: &mut Environment,
        s1: MutPtr<T>,
        s2: ConstPtr<T>,
        n: GuestUSize,
    ) -> MutPtr<T> {
        let s1end = s1 + Self::strlen(env, s1.cast_const());
        for i in 0..n {
            let c = env.mem.read(s2 + i);
            env.mem.write(s1end + i, c);
            if c == Self::null() {
                return s1;
            }
        }
        env.mem.write(s1end + n, Self::null());
        s1
    }

    pub(super) fn strstr(
        env: &mut Environment,
        string: ConstPtr<T>,
        substring: ConstPtr<T>,
    ) -> ConstPtr<T> {
        let mut offset = 0;
        loop {
            let mut inner_offset = 0;
            loop {
                let char_string = env.mem.read(string + offset + inner_offset);
                let char_substring = env.mem.read(substring + inner_offset);
                if char_substring == Self::null() {
                    return string + offset;
                } else if char_string == Self::null() {
                    return Ptr::null();
                } else if char_string != char_substring {
                    break;
                } else {
                    inner_offset += 1;
                }
            }
            offset += 1;
        }
    }

    pub(super) fn strchr(env: &mut Environment, string: ConstPtr<T>, char: T) -> ConstPtr<T> {
        let len = Self::strlen(env, string);
        let mut offset = 0;
        loop {
            if env.mem.read(string + offset) == char {
                return string + offset;
            }
            if offset == len {
                return Ptr::null();
            }
            offset += 1;
        }
    }

    pub(super) fn strrchr(env: &mut Environment, string: ConstPtr<T>, char: T) -> ConstPtr<T> {
        let mut offset = Self::strlen(env, string);
        loop {
            if env.mem.read(string + offset) == char {
                return string + offset;
            }
            if offset == 0 {
                return Ptr::null();
            }
            offset -= 1;
        }
    }

    pub(super) fn strpbrk(
        env: &mut Environment,
        string: ConstPtr<T>,
        charset: ConstPtr<T>,
    ) -> ConstPtr<T> {
        let mut i = 0;
        loop {
            let c = env.mem.read(string + i);
            if c == Self::null() {
                return Ptr::null();
            }
            let mut j = 0;
            loop {
                let cc = env.mem.read(charset + j);
                if cc == Self::null() {
                    break;
                }
                if c == cc {
                    return string + i;
                }
                j += 1;
            }
            i += 1;
        }
    }

    pub(super) fn strlcpy(
        env: &mut Environment,
        dst: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
    ) -> GuestUSize {
        let mut i = 0;
        loop {
            let c = env.mem.read(src + i);
            match i.cmp(&(size - 1)) {
                Ordering::Less => env.mem.write(dst + i, c),
                Ordering::Equal => env.mem.write(dst + i, Self::null()),
                _ => {}
            }

            if c == Self::null() {
                break;
            }
            i += 1;
        }
        i
    }

    // --- Added missing functions below ---

    pub(super) fn strlcat(
        env: &mut Environment,
        dst: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
    ) -> GuestUSize {
        let dst_len = Self::strlen(env, dst.cast_const());
        let src_len = Self::strlen(env, src);
        if dst_len >= size {
            return size + src_len;
        }
        let copy_len = (size - dst_len - 1).min(src_len);
        for i in 0..copy_len {
            let c = env.mem.read(src + i);
            env.mem.write(dst + dst_len + i, c);
        }
        env.mem.write(dst + dst_len + copy_len, Self::null());
        dst_len + src_len
    }

    pub(super) fn strspn(env: &mut Environment, s: ConstPtr<T>, accept: ConstPtr<T>) -> GuestUSize {
        let mut i = 0;
        loop {
            let c = env.mem.read(s + i);
            if c == Self::null() {
                break;
            }
            let mut j = 0;
            let mut found = false;
            loop {
                let a = env.mem.read(accept + j);
                if a == Self::null() {
                    break;
                }
                if c == a {
                    found = true;
                    break;
                }
                j += 1;
            }
            if !found {
                break;
            }
            i += 1;
        }
        i
    }

    pub(super) fn strpbrk(
        env: &mut Environment,
        s: ConstPtr<T>,
        accept: ConstPtr<T>,
    ) -> ConstPtr<T> {
        let mut i = 0;
        loop {
            let c = env.mem.read(s + i);
            if c == Self::null() {
                return Ptr::null();
            }
            let mut j = 0;
            loop {
                let a = env.mem.read(accept + j);
                if a == Self::null() {
                    break;
                }
                if c == a {
                    return s + i;
                }
                j += 1;
            }
            i += 1;
        }
    }
}
