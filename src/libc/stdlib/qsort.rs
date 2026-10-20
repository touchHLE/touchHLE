/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! stdlib's qsort

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutPtr;
use crate::Environment;

fn qsort(
    env: &mut Environment,
    base: MutPtr<u8>,
    nitems: u32,
    size: u32,
    compar: GuestFunction, // int (*compar)(const void *, const void*))
) {
    qsort_generic(
        env,
        nitems,
        &mut |env, i, j| {
            let i_ptr = base + i * size;
            let j_ptr = base + j * size;
            compar.call_from_host(env, (i_ptr.cast_const(), j_ptr.cast_const()))
        },
        &mut |env: &mut Environment, mut i: u32, mut j: u32| {
            if i == j {
                return;
            }
            if i > j {
                std::mem::swap(&mut i, &mut j);
            }
            let base_slice = env.mem.bytes_at_mut(base, nitems * size);
            let (_, slice_after_first) = base_slice.split_at_mut((i * size) as usize);
            let offset = (j * size - i * size) as usize;
            let (left, right) = slice_after_first.split_at_mut(offset);
            left[..size as usize].swap_with_slice(&mut right[..size as usize]);
        },
    );
}

pub fn qsort_generic<T, Compare, Swap>(
    user_data: &mut T,
    nitems: u32,
    compar: &mut Compare, // int (*compar)(const void *, const void*))
    swap: &mut Swap,
) where
    Compare: FnMut(&mut T, u32, u32) -> i32,
    Swap: FnMut(&mut T, u32, u32),
{
    if nitems < 2 {
        return;
    }
    qsort_rec(user_data, compar, swap, 0, nitems - 1);
}

fn qsort_rec<T, Compare, Swap>(
    user_data: &mut T,
    compar: &mut Compare, // int (*compar)(const void *, const void*))
    swap: &mut Swap,
    low: u32,
    hi: u32,
) where
    Compare: FnMut(&mut T, u32, u32) -> i32,
    Swap: FnMut(&mut T, u32, u32),
{
    if low >= hi {
        return;
    }
    // TODO: use median selection
    let pivot = low;
    let mut separator = low + 1;
    for i in low + 1..=hi {
        if compar(user_data, i, pivot) < 0 {
            swap(user_data, i, separator);
            separator += 1;
        }
    }
    swap(user_data, pivot, separator - 1);
    if separator > 1 {
        qsort_rec(user_data, compar, swap, low, separator - 2);
    }
    qsort_rec(user_data, compar, swap, separator, hi);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(qsort(_, _, _, _))];
