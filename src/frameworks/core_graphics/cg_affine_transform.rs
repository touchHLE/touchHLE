/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGAffineTransform.h`

use super::CGFloat;
use crate::dyld::{ConstantExports, HostConstant};
use crate::mem::SafeRead;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
/// 3-by-3 matrix type where the columns are `[a, c, tx]`, `[b, d, ty]`,
/// `[0, 0, 1]`.
pub struct CGAffineTransform {
    pub a: CGFloat,
    pub b: CGFloat,
    pub c: CGFloat,
    pub d: CGFloat,
    pub tx: CGFloat,
    pub ty: CGFloat,
}
unsafe impl SafeRead for CGAffineTransform {}

#[rustfmt::skip]
pub const CGAffineTransformIdentity: CGAffineTransform = CGAffineTransform {
    a: 1.0, c: 0.0, tx: 0.0,
    b: 0.0, d: 1.0, ty: 0.0,
    // 0.0, 0.0, 1.0,
};

pub const CONSTANTS: ConstantExports = &[(
    "_CGAffineTransformIdentity",
    HostConstant::Custom(|mem| {
        mem.alloc_and_write(CGAffineTransformIdentity)
            .cast()
            .cast_const()
    }),
)];
