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
use std::ops::Add;

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
    ) -> MutPtr<T> {
        for i in 0..count {
            env.mem.write(dest + i, ch);
        }
        dest
    }

    pub(super) fn memcpy(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
    ) -> MutPtr<T> {
        env.mem
            .memmove(dest.cast(), src.cast(), size * guest_size_of::<T>());
        dest
    }

    pub(super) fn memmove(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
    ) -> MutPtr<T> {
        env.mem
            .memmove(dest.cast(), src.cast(), size * guest_size_of::<T>());
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

            // TODO: While the C standard only requires this value to be
            // non-zero and have the right sign, the man pages for iOS say this
            // value should have a magnitude corresponding to the difference
            // between the first differing bytes. Maybe some app relies on that?
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

    pub(super) fn strcpy(env: &mut Environment, dest: MutPtr<T>, src: ConstPtr<T>) -> MutPtr<T> {
        {
            let (mut dest, mut src) = (dest, src);
            loop {
                let c = env.mem.read(src);
                env.mem.write(dest, c);
                if c == Self::null() {
                    break;
                }
                dest += 1;
                src += 1;
            }
        }
        dest
    }
    pub(super) fn strcat(env: &mut Environment, dest: MutPtr<T>, src: ConstPtr<T>) -> MutPtr<T> {
        {
            let dest = dest + Self::strlen(env, dest.cast_const());
            Self::strcpy(env, dest, src);
        }
        dest
    }

    pub(super) fn strncpy(
        env: &mut Environment,
        dest: MutPtr<T>,
        src: ConstPtr<T>,
        size: GuestUSize,
    ) -> MutPtr<T> {
        let mut end = false;
        for i in 0..size {
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
        Self::strcpy(env, new, src)
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
        s1: ConstPtr<T>,
        s2: ConstPtr<T>,
        n: GuestUSize,
    ) -> ConstPtr<T> {
        let len1 = Self::strlen(env, s1);
        let len2 = Self::strlen(env, s2);

        let lenToAdd = if len2 < n { len2 } else { n };

        Self::strncpy(env, s1.add(len1).cast_mut(), s2, lenToAdd);
        env.mem
            .write(s1.add(len1 + lenToAdd).cast_mut(), Self::null());
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
        let mut offset = 0;
        loop {
            if offset == Self::strlen(env, string) {
                return Ptr::null();
            }
            offset += 1;
            if env.mem.read(string + offset) == char {
                return string + offset;
            }
        }
    }

    pub(super) fn strrchr(env: &mut Environment, string: ConstPtr<T>, char: T) -> ConstPtr<T> {
        let mut offset = Self::strlen(env, string);
        loop {
            if offset == 0 {
                return Ptr::null();
            }
            offset -= 1;
            if env.mem.read(string + offset) == char {
                return string + offset;
            }
        }
    }
}
