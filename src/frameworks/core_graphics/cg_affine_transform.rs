/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGAffineTransform.h`

use super::{CGFloat, CGPoint, CGRect, CGSize};
use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::matrix::Matrix;
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
impl_GuestRet_for_large_struct!(CGAffineTransform);

// These conversions allow sharing code with the touchHLE Matrix type.
impl TryFrom<Matrix<3>> for CGAffineTransform {
    type Error = ();

    fn try_from(value: Matrix<3>) -> Result<CGAffineTransform, ()> {
        let columns = value.columns();
        if columns[2] == [0.0, 0.0, 1.0] {
            Ok(CGAffineTransform {
                a: columns[0][0],
                b: columns[1][0],
                c: columns[0][1],
                d: columns[1][1],
                tx: columns[0][2],
                ty: columns[1][2],
            })
        } else {
            Err(())
        }
    }
}
impl From<CGAffineTransform> for Matrix<3> {
    fn from(value: CGAffineTransform) -> Matrix<3> {
        let CGAffineTransform { a, b, c, d, tx, ty } = value;
        Matrix::<3>::from_columns([[a, c, tx], [b, d, ty], [0.0, 0.0, 1.0]])
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

fn CGAffineTransformMake(
    _env: &mut Environment,
    a: CGFloat,
    b: CGFloat,
    c: CGFloat,
    d: CGFloat,
    tx: CGFloat,
    ty: CGFloat,
) -> CGAffineTransform {
    CGAffineTransform { a, b, c, d, tx, ty }
}

fn CGAffineTransformMakeRotation(_env: &mut Environment, angle: CGFloat) -> CGAffineTransform {
    Matrix::<3>::from(&Matrix::<2>::z_rotation(angle))
        .try_into()
        .unwrap()
}
fn CGAffineTransformMakeScale(_env: &mut Environment, x: CGFloat, y: CGFloat) -> CGAffineTransform {
    Matrix::<3>::from(&Matrix::<2>::scale_2d(x, y))
        .try_into()
        .unwrap()
}
fn CGAffineTransformMakeTranslation(
    _env: &mut Environment,
    x: CGFloat,
    y: CGFloat,
) -> CGAffineTransform {
    Matrix::<3>::translate_2d(x, y).try_into().unwrap()
}

fn CGAffineTransformConcat(
    _env: &mut Environment,
    a: CGAffineTransform,
    b: CGAffineTransform,
) -> CGAffineTransform {
    Matrix::<3>::multiply(&a.into(), &b.into())
        .try_into()
        .unwrap()
}

fn CGAffineTransformRotate(
    env: &mut Environment,
    existing: CGAffineTransform,
    angle: CGFloat,
) -> CGAffineTransform {
    let t = CGAffineTransformMakeRotation(env, angle);
    CGAffineTransformConcat(env, existing, t)
}
fn CGAffineTransformScale(
    env: &mut Environment,
    existing: CGAffineTransform,
    x: CGFloat,
    y: CGFloat,
) -> CGAffineTransform {
    let t = CGAffineTransformMakeScale(env, x, y);
    CGAffineTransformConcat(env, existing, t)
}
fn CGAffineTransformTranslate(
    env: &mut Environment,
    existing: CGAffineTransform,
    x: CGFloat,
    y: CGFloat,
) -> CGAffineTransform {
    let t = CGAffineTransformMakeTranslation(env, x, y);
    CGAffineTransformConcat(env, existing, t)
}

fn CGPointApplyAffineTransform(
    _env: &mut Environment,
    point: CGPoint,
    transform: CGAffineTransform,
) -> CGPoint {
    let [x, y, _z] = Matrix::<3>::transform(&transform.into(), [point.x, point.y, 0.0]);
    CGPoint { x, y }
}
fn CGSizeApplyAffineTransform(
    _env: &mut Environment,
    rect: CGSize,
    transform: CGAffineTransform,
) -> CGSize {
    let [width, height, _depth] =
        Matrix::<3>::transform(&transform.into(), [rect.width, rect.height, 0.0]);
    CGSize { width, height }
}
fn CGRectApplyAffineTransform(
    env: &mut Environment,
    rect: CGRect,
    transform: CGAffineTransform,
) -> CGRect {
    // Affine transforms applied to a rectangle don't necessarily return a
    // rectangle (just a quadrilateral), so CGRectApplyAffineTransform
    // essentially returns the bounding box of the points.

    let corner1 = rect.origin;
    let corner2 = CGPoint {
        x: rect.origin.x + rect.size.width,
        y: rect.origin.y,
    };
    let corner3 = CGPoint {
        x: rect.origin.x,
        y: rect.origin.y + rect.size.height,
    };
    let corner4 = CGPoint {
        x: rect.origin.x + rect.size.width,
        y: rect.origin.y + rect.size.height,
    };

    let point1 = CGPointApplyAffineTransform(env, corner1, transform);
    let point2 = CGPointApplyAffineTransform(env, corner2, transform);
    let point3 = CGPointApplyAffineTransform(env, corner3, transform);
    let point4 = CGPointApplyAffineTransform(env, corner4, transform);

    let x1 = point1.x.min(point2.x).min(point3.x).min(point4.x);
    let x2 = point1.x.max(point2.x).max(point3.x).max(point4.x);
    let y1 = point1.y.min(point2.y).min(point3.y).min(point4.y);
    let y2 = point1.y.max(point2.y).max(point3.y).max(point4.y);

    CGRect {
        origin: CGPoint { x: x1, y: y1 },
        size: CGSize {
            width: x2 - x1,
            height: y2 - y1,
        },
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGAffineTransformIsIdentity(_)),
    export_c_func!(CGAffineTransformEqualToTransform(_, _)),
    export_c_func!(CGAffineTransformMake(_, _, _, _, _, _)),
    export_c_func!(CGAffineTransformMakeRotation(_)),
    export_c_func!(CGAffineTransformMakeScale(_, _)),
    export_c_func!(CGAffineTransformMakeTranslation(_, _)),
    export_c_func!(CGAffineTransformConcat(_, _)),
    export_c_func!(CGAffineTransformRotate(_, _)),
    export_c_func!(CGAffineTransformScale(_, _, _)),
    export_c_func!(CGAffineTransformTranslate(_, _, _)),
    export_c_func!(CGPointApplyAffineTransform(_, _)),
    export_c_func!(CGSizeApplyAffineTransform(_, _)),
    export_c_func!(CGRectApplyAffineTransform(_, _)),
];
