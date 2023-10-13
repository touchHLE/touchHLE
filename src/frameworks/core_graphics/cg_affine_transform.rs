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
use crate::Environment;

#[derive(Copy, Clone, Debug, PartialEq)]
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
        self.a.to_regs(&mut regs[0..1]);
        self.b.to_regs(&mut regs[1..2]);
        self.c.to_regs(&mut regs[2..3]);
        self.d.to_regs(&mut regs[3..4]);
        self.tx.to_regs(&mut regs[4..5]);
        self.ty.to_regs(&mut regs[5..6]);
    }
}

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

fn CGAffineTransformEqualToTransform(
    _env: &mut Environment,
    a: CGAffineTransform,
    b: CGAffineTransform,
) -> bool {
    a == b
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGAffineTransformIsIdentity(_)),
    export_c_func!(CGAffineTransformEqualToTransform(_, _)),
];
