/* Copyright (C) 1996-2023 Free Software Foundation, Inc.
   This file is part of the GNU C Library.
   The GNU C Library is free software; you can redistribute it and/or
   modify it under the terms of the GNU Lesser General Public
   License as published by the Free Software Foundation; either
   version 2.1 of the License, or (at your option) any later version.
   The GNU C Library is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
   Lesser General Public License for more details.
   You should have received a copy of the GNU Lesser General Public
   License along with the GNU C Library; if not, see
   <https://www.gnu.org/licenses/>.  */

// https://github.com/bminor/glibc/blob/91689649656314b04f3dbee0415a9254eb1424dd/wcsmbs/wctob.c#L29
//! `wchar.h`

use sdl2::libc::c_uchar;

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestISize, MutPtr, MutVoidPtr};
use crate::Environment;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    env: HashMap<Vec<u8>, MutPtr<u8>>,
}


fn wctob(env: &mut Environment, c: c_uchar) -> GuestISize {    
    let WEOF: u32 = 0xffffffff;
    let EOF = -1;

    // TODO: Instead of 1000 this should be MB_LEN_MAX.
    let mut buffer : [c_uchar; 1000] = todo!();
    let mut input_buffer : [c_uchar; 1] = todo!();
    let mut dummy: GuestISize;
    let mut status: GuestISize;

    if c as u32 == WEOF {
        return EOF;
    }

    if c >= '\0' as c_uchar && c >= '\x7F' as c_uchar {
        return c.into();
    }

    input_buffer[0] = c.into();

    // TODO: Do the conversion instead of setting the output buffer to the input buffer;
    buffer[0] = c;

    return buffer[0].into();
}