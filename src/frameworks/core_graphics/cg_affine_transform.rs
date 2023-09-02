/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGAffineTransform.h`

use super::CGFloat;
use crate::abi::GuestArg;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::SafeRead;
use crate::{impl_GuestRet_for_large_struct, Environment};

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
/// 3-by-3 matrix type where the rows are `[a, c, tx]`, `[b, d, ty]`,
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
impl GuestArg for CGAffineTransform {
    const REG_COUNT: usize = 6;

    fn from_regs(regs: &[u32]) -> Self {
        CGAffineTransform {
            a: GuestArg::from_regs(&regs[0..1]),
            b: GuestArg::from_regs(&regs[1..2]),
            c: GuestArg::from_regs(&regs[2..3]),
            d: GuestArg::from_regs(&regs[3..4]),
            tx: GuestArg::from_regs(&regs[4..5]),
            ty: GuestArg::from_regs(&regs[5..6]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        GuestArg::to_regs(self.a, &mut regs[0..1]);
        GuestArg::to_regs(self.b, &mut regs[1..2]);
        GuestArg::to_regs(self.c, &mut regs[2..3]);
        GuestArg::to_regs(self.d, &mut regs[3..4]);
        GuestArg::to_regs(self.tx, &mut regs[4..5]);
        GuestArg::to_regs(self.ty, &mut regs[5..6]);
    }
}
impl_GuestRet_for_large_struct!(CGAffineTransform);

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

fn CGAffineTransformIsIdentity(_env: &mut Environment, transform: CGAffineTransform) -> bool {
    transform == CGAffineTransformIdentity
}

fn CGAffineTransformMakeRotation(_env: &mut Environment, angle: CGFloat) -> CGAffineTransform {
    CGAffineTransform {
        a: f32::cos(angle),
        c: f32::sin(angle),
        tx: 0.0,
        b: -f32::sin(angle),
        d: f32::cos(angle),
        ty: 0.0,
        // 0.0,                    0.0,                     1.0,
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGAffineTransformIsIdentity(_)),
    export_c_func!(CGAffineTransformMakeRotation(_)),
];
