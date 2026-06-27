/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CATransform3D` and the associated C API.
//!
//! Apple documents `CATransform3D` as a column-major 4×4 matrix: `m11`..`m14`
//! form the first column, `m21`..`m24` the second, and so on. Translations
//! therefore live in `m41`, `m42`, `m43`, which is the standard OpenGL
//! convention.
//!
//! The C API surface mirrors Apple's `<QuartzCore/CATransform3D.h>`. See:
//! - [CATransform3D Reference](https://developer.apple.com/documentation/quartzcore/catransform3d)

use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_graphics::cg_affine_transform::{
    CGAffineTransform, CGAffineTransformIdentity,
};
use crate::frameworks::core_graphics::CGFloat;
use crate::matrix::Matrix;
use crate::mem::SafeRead;
use crate::Environment;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(C, packed)]
pub struct CATransform3D {
    pub m11: CGFloat,
    pub m12: CGFloat,
    pub m13: CGFloat,
    pub m14: CGFloat,
    pub m21: CGFloat,
    pub m22: CGFloat,
    pub m23: CGFloat,
    pub m24: CGFloat,
    pub m31: CGFloat,
    pub m32: CGFloat,
    pub m33: CGFloat,
    pub m34: CGFloat,
    pub m41: CGFloat,
    pub m42: CGFloat,
    pub m43: CGFloat,
    pub m44: CGFloat,
}

unsafe impl SafeRead for CATransform3D {}

impl GuestArg for CATransform3D {
    const REG_COUNT: usize = 16;

    fn from_regs(regs: &[u32]) -> Self {
        CATransform3D {
            m11: GuestArg::from_regs(&regs[0..1]),
            m12: GuestArg::from_regs(&regs[1..2]),
            m13: GuestArg::from_regs(&regs[2..3]),
            m14: GuestArg::from_regs(&regs[3..4]),
            m21: GuestArg::from_regs(&regs[4..5]),
            m22: GuestArg::from_regs(&regs[5..6]),
            m23: GuestArg::from_regs(&regs[6..7]),
            m24: GuestArg::from_regs(&regs[7..8]),
            m31: GuestArg::from_regs(&regs[8..9]),
            m32: GuestArg::from_regs(&regs[9..10]),
            m33: GuestArg::from_regs(&regs[10..11]),
            m34: GuestArg::from_regs(&regs[11..12]),
            m41: GuestArg::from_regs(&regs[12..13]),
            m42: GuestArg::from_regs(&regs[13..14]),
            m43: GuestArg::from_regs(&regs[14..15]),
            m44: GuestArg::from_regs(&regs[15..16]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        // Copy fields to locals first; `#[repr(C, packed)]` makes taking a
        // reference to a field undefined behaviour even on aligned data.
        let (m11, m12, m13, m14) = (self.m11, self.m12, self.m13, self.m14);
        let (m21, m22, m23, m24) = (self.m21, self.m22, self.m23, self.m24);
        let (m31, m32, m33, m34) = (self.m31, self.m32, self.m33, self.m34);
        let (m41, m42, m43, m44) = (self.m41, self.m42, self.m43, self.m44);
        m11.to_regs(&mut regs[0..1]);
        m12.to_regs(&mut regs[1..2]);
        m13.to_regs(&mut regs[2..3]);
        m14.to_regs(&mut regs[3..4]);
        m21.to_regs(&mut regs[4..5]);
        m22.to_regs(&mut regs[5..6]);
        m23.to_regs(&mut regs[6..7]);
        m24.to_regs(&mut regs[7..8]);
        m31.to_regs(&mut regs[8..9]);
        m32.to_regs(&mut regs[9..10]);
        m33.to_regs(&mut regs[10..11]);
        m34.to_regs(&mut regs[11..12]);
        m41.to_regs(&mut regs[12..13]);
        m42.to_regs(&mut regs[13..14]);
        m43.to_regs(&mut regs[14..15]);
        m44.to_regs(&mut regs[15..16]);
    }
}

impl_GuestRet_for_large_struct!(CATransform3D);

// MARK: - Conversions to/from the generic Matrix<4>

impl From<CATransform3D> for Matrix<4> {
    fn from(value: CATransform3D) -> Matrix<4> {
        let (m11, m12, m13, m14) = (value.m11, value.m12, value.m13, value.m14);
        let (m21, m22, m23, m24) = (value.m21, value.m22, value.m23, value.m24);
        let (m31, m32, m33, m34) = (value.m31, value.m32, value.m33, value.m34);
        let (m41, m42, m43, m44) = (value.m41, value.m42, value.m43, value.m44);
        Matrix::<4>::from_columns([
            [m11, m12, m13, m14],
            [m21, m22, m23, m24],
            [m31, m32, m33, m34],
            [m41, m42, m43, m44],
        ])
    }
}

impl From<Matrix<4>> for CATransform3D {
    fn from(matrix: Matrix<4>) -> Self {
        let c = matrix.columns();
        CATransform3D {
            m11: c[0][0],
            m12: c[0][1],
            m13: c[0][2],
            m14: c[0][3],
            m21: c[1][0],
            m22: c[1][1],
            m23: c[1][2],
            m24: c[1][3],
            m31: c[2][0],
            m32: c[2][1],
            m33: c[2][2],
            m34: c[2][3],
            m41: c[3][0],
            m42: c[3][1],
            m43: c[3][2],
            m44: c[3][3],
        }
    }
}

// MARK: - Identity constant

#[rustfmt::skip]
pub const CATransform3DIdentity: CATransform3D = CATransform3D {
    m11: 1.0, m12: 0.0, m13: 0.0, m14: 0.0,
    m21: 0.0, m22: 1.0, m23: 0.0, m24: 0.0,
    m31: 0.0, m32: 0.0, m33: 1.0, m34: 0.0,
    m41: 0.0, m42: 0.0, m43: 0.0, m44: 1.0,
};

pub const CONSTANTS: ConstantExports = &[(
    "_CATransform3DIdentity",
    HostConstant::Custom(|env| {
        env.mem
            .alloc_and_write(CATransform3DIdentity)
            .cast()
            .cast_const()
    }),
)];

// MARK: - Operations on CATransform3D

impl CATransform3D {
    pub fn equal_to(self, other: Self) -> bool {
        let (a11, a12, a13, a14) = (self.m11, self.m12, self.m13, self.m14);
        let (a21, a22, a23, a24) = (self.m21, self.m22, self.m23, self.m24);
        let (a31, a32, a33, a34) = (self.m31, self.m32, self.m33, self.m34);
        let (a41, a42, a43, a44) = (self.m41, self.m42, self.m43, self.m44);
        let (b11, b12, b13, b14) = (other.m11, other.m12, other.m13, other.m14);
        let (b21, b22, b23, b24) = (other.m21, other.m22, other.m23, other.m24);
        let (b31, b32, b33, b34) = (other.m31, other.m32, other.m33, other.m34);
        let (b41, b42, b43, b44) = (other.m41, other.m42, other.m43, other.m44);
        a11 == b11
            && a12 == b12
            && a13 == b13
            && a14 == b14
            && a21 == b21
            && a22 == b22
            && a23 == b23
            && a24 == b24
            && a31 == b31
            && a32 == b32
            && a33 == b33
            && a34 == b34
            && a41 == b41
            && a42 == b42
            && a43 == b43
            && a44 == b44
    }

    pub fn is_identity(self) -> bool {
        self.equal_to(CATransform3DIdentity)
    }

    pub fn make_translation(tx: CGFloat, ty: CGFloat, tz: CGFloat) -> Self {
        let mut t = CATransform3DIdentity;
        t.m41 = tx;
        t.m42 = ty;
        t.m43 = tz;
        t
    }

    pub fn make_scale(sx: CGFloat, sy: CGFloat, sz: CGFloat) -> Self {
        let mut t = CATransform3DIdentity;
        t.m11 = sx;
        t.m22 = sy;
        t.m33 = sz;
        t
    }

    /// Build a rotation of `angle` radians about the axis `(x, y, z)`.
    ///
    /// This matches the formula given in Apple's `CATransform3DMakeRotation`
    /// documentation: a non-unit axis is normalised, and a zero-length axis
    /// yields the identity matrix.
    pub fn make_rotation(angle: CGFloat, x: CGFloat, y: CGFloat, z: CGFloat) -> Self {
        let length = (x * x + y * y + z * z).sqrt();
        if length == 0.0 {
            return CATransform3DIdentity;
        }
        let nx = x / length;
        let ny = y / length;
        let nz = z / length;
        let c = angle.cos();
        let s = angle.sin();
        let t = 1.0 - c;
        CATransform3D {
            m11: t * nx * nx + c,
            m12: t * nx * ny + nz * s,
            m13: t * nx * nz - ny * s,
            m14: 0.0,

            m21: t * nx * ny - nz * s,
            m22: t * ny * ny + c,
            m23: t * ny * nz + nx * s,
            m24: 0.0,

            m31: t * nx * nz + ny * s,
            m32: t * ny * nz - nx * s,
            m33: t * nz * nz + c,
            m34: 0.0,

            m41: 0.0,
            m42: 0.0,
            m43: 0.0,
            m44: 1.0,
        }
    }

    /// Equivalent to `CATransform3DConcat(a, b)`: combines two transforms so
    /// that the result is `a * b` in Apple's row-vector convention. Concretely,
    /// applying the result to a row vector `v` is equivalent to first applying
    /// `a` and then `b`.
    pub fn concat(self, other: Self) -> Self {
        // Apple stores `CATransform3D` in row-major layout (`m11`..`m14` is
        // the first row), but our `Matrix<4>` is column-major. Each
        // `CATransform3D <-> Matrix<4>` conversion therefore implicitly
        // transposes. With the row-vector `a * b` semantics Apple documents,
        // the column-major equivalent is `B_col * A_col`, and our
        // `Matrix::multiply(x, y)` happens to compute `y * x` (see the
        // doc comment on `Matrix::multiply`), so calling it as `(a, b)` gives
        // the right answer.
        let a: Matrix<4> = self.into();
        let b: Matrix<4> = other.into();
        Matrix::<4>::multiply(&a, &b).into()
    }

    /// Equivalent to `CATransform3DTranslate(t, tx, ty, tz)`, which Apple
    /// documents as `translate(tx, ty, tz) * t` (row-vector convention).
    pub fn translate(self, tx: CGFloat, ty: CGFloat, tz: CGFloat) -> Self {
        Self::make_translation(tx, ty, tz).concat(self)
    }

    /// Equivalent to `CATransform3DScale(t, sx, sy, sz)`.
    pub fn scale(self, sx: CGFloat, sy: CGFloat, sz: CGFloat) -> Self {
        Self::make_scale(sx, sy, sz).concat(self)
    }

    /// Equivalent to `CATransform3DRotate(t, angle, x, y, z)`.
    pub fn rotate(self, angle: CGFloat, x: CGFloat, y: CGFloat, z: CGFloat) -> Self {
        Self::make_rotation(angle, x, y, z).concat(self)
    }

    /// Invert a transform. Returns the original transform if it is singular,
    /// matching `CATransform3DInvert`'s documented behaviour ("Returns the
    /// original matrix if `t` has no inverse").
    pub fn invert(self) -> Self {
        let m: Matrix<4> = self.into();
        match m.inverse() {
            Some(inv) => inv.into(),
            None => self,
        }
    }

    /// Project a 2-D `CGAffineTransform` into the equivalent 4×4 transform.
    pub fn from_affine(t: CGAffineTransform) -> Self {
        let (a, b, c, d, tx, ty) = (t.a, t.b, t.c, t.d, t.tx, t.ty);
        CATransform3D {
            m11: a,
            m12: b,
            m13: 0.0,
            m14: 0.0,
            m21: c,
            m22: d,
            m23: 0.0,
            m24: 0.0,
            m31: 0.0,
            m32: 0.0,
            m33: 1.0,
            m34: 0.0,
            m41: tx,
            m42: ty,
            m43: 0.0,
            m44: 1.0,
        }
    }

    /// Returns whether the transform can be exactly represented by a 2-D
    /// `CGAffineTransform`. This matches `CATransform3DIsAffine`.
    pub fn is_affine(self) -> bool {
        let (m13, m14) = (self.m13, self.m14);
        let (m23, m24) = (self.m23, self.m24);
        let (m31, m32, m33, m34) = (self.m31, self.m32, self.m33, self.m34);
        let (m43, m44) = (self.m43, self.m44);
        m13 == 0.0
            && m14 == 0.0
            && m23 == 0.0
            && m24 == 0.0
            && m31 == 0.0
            && m32 == 0.0
            && m33 == 1.0
            && m34 == 0.0
            && m43 == 0.0
            && m44 == 1.0
    }

    /// Project to a 2-D `CGAffineTransform`. The z components are discarded;
    /// this matches `CATransform3DGetAffineTransform`, which is documented as
    /// returning an undefined affine transform if the input is not actually
    /// affine.
    pub fn to_affine(self) -> CGAffineTransform {
        CGAffineTransform {
            a: self.m11,
            b: self.m12,
            c: self.m21,
            d: self.m22,
            tx: self.m41,
            ty: self.m42,
        }
    }
}

// MARK: - C API

fn CATransform3DIsIdentity(_env: &mut Environment, t: CATransform3D) -> bool {
    t.is_identity()
}

fn CATransform3DEqualToTransform(
    _env: &mut Environment,
    a: CATransform3D,
    b: CATransform3D,
) -> bool {
    a.equal_to(b)
}

fn CATransform3DMakeTranslation(
    _env: &mut Environment,
    tx: CGFloat,
    ty: CGFloat,
    tz: CGFloat,
) -> CATransform3D {
    CATransform3D::make_translation(tx, ty, tz)
}

fn CATransform3DMakeScale(
    _env: &mut Environment,
    sx: CGFloat,
    sy: CGFloat,
    sz: CGFloat,
) -> CATransform3D {
    CATransform3D::make_scale(sx, sy, sz)
}

fn CATransform3DMakeRotation(
    _env: &mut Environment,
    angle: CGFloat,
    x: CGFloat,
    y: CGFloat,
    z: CGFloat,
) -> CATransform3D {
    CATransform3D::make_rotation(angle, x, y, z)
}

fn CATransform3DTranslate(
    _env: &mut Environment,
    t: CATransform3D,
    tx: CGFloat,
    ty: CGFloat,
    tz: CGFloat,
) -> CATransform3D {
    t.translate(tx, ty, tz)
}

fn CATransform3DScale(
    _env: &mut Environment,
    t: CATransform3D,
    sx: CGFloat,
    sy: CGFloat,
    sz: CGFloat,
) -> CATransform3D {
    t.scale(sx, sy, sz)
}

fn CATransform3DRotate(
    _env: &mut Environment,
    t: CATransform3D,
    angle: CGFloat,
    x: CGFloat,
    y: CGFloat,
    z: CGFloat,
) -> CATransform3D {
    t.rotate(angle, x, y, z)
}

fn CATransform3DConcat(
    _env: &mut Environment,
    a: CATransform3D,
    b: CATransform3D,
) -> CATransform3D {
    a.concat(b)
}

fn CATransform3DInvert(_env: &mut Environment, t: CATransform3D) -> CATransform3D {
    t.invert()
}

fn CATransform3DMakeAffineTransform(
    _env: &mut Environment,
    affine: CGAffineTransform,
) -> CATransform3D {
    CATransform3D::from_affine(affine)
}

fn CATransform3DIsAffine(_env: &mut Environment, t: CATransform3D) -> bool {
    t.is_affine()
}

fn CATransform3DGetAffineTransform(
    _env: &mut Environment,
    t: CATransform3D,
) -> CGAffineTransform {
    if t.is_affine() {
        t.to_affine()
    } else {
        // Apple documents the result as "undefined" if the input isn't affine.
        // Falling back to identity is safer than propagating garbage z values
        // into 2D-only code paths.
        CGAffineTransformIdentity
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CATransform3DIsIdentity(_)),
    export_c_func!(CATransform3DEqualToTransform(_, _)),
    export_c_func!(CATransform3DMakeTranslation(_, _, _)),
    export_c_func!(CATransform3DMakeScale(_, _, _)),
    export_c_func!(CATransform3DMakeRotation(_, _, _, _)),
    export_c_func!(CATransform3DTranslate(_, _, _, _)),
    export_c_func!(CATransform3DScale(_, _, _, _)),
    export_c_func!(CATransform3DRotate(_, _, _, _, _)),
    export_c_func!(CATransform3DConcat(_, _)),
    export_c_func!(CATransform3DInvert(_)),
    export_c_func!(CATransform3DMakeAffineTransform(_)),
    export_c_func!(CATransform3DIsAffine(_)),
    export_c_func!(CATransform3DGetAffineTransform(_)),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: CATransform3D, b: CATransform3D, eps: f32) -> bool {
        let fields = [
            (a.m11, b.m11),
            (a.m12, b.m12),
            (a.m13, b.m13),
            (a.m14, b.m14),
            (a.m21, b.m21),
            (a.m22, b.m22),
            (a.m23, b.m23),
            (a.m24, b.m24),
            (a.m31, b.m31),
            (a.m32, b.m32),
            (a.m33, b.m33),
            (a.m34, b.m34),
            (a.m41, b.m41),
            (a.m42, b.m42),
            (a.m43, b.m43),
            (a.m44, b.m44),
        ];
        fields.iter().all(|(x, y)| (x - y).abs() <= eps)
    }

    #[test]
    fn identity_is_identity() {
        assert!(CATransform3DIdentity.is_identity());
        assert!(!CATransform3D::make_translation(1.0, 0.0, 0.0).is_identity());
    }

    #[test]
    fn translate_then_scale_is_consistent() {
        // Apple defines `CATransform3DConcat(t, s)` as `t * s` in row-vector
        // convention: row vector v becomes v·t·s, i.e. translate first, then
        // scale. For v = (0,0,0,1), that gives (0,0,0,1)·t = (10,20,30,1)
        // then ·s = (20,60,120,1).
        let t = CATransform3D::make_translation(10.0, 20.0, 30.0);
        let s = CATransform3D::make_scale(2.0, 3.0, 4.0);
        let combined = t.concat(s);
        assert!((combined.m41 - 20.0).abs() < 1e-5);
        assert!((combined.m42 - 60.0).abs() < 1e-5);
        assert!((combined.m43 - 120.0).abs() < 1e-5);
    }

    #[test]
    fn translate_method_matches_apple_semantics() {
        // CATransform3DTranslate(t, tx, ty, tz) = translate * t in Apple's
        // row-vector convention. For v = (0,0,0,1) the result is
        // v * translate * scale = (tx, ty, tz, 1) * scale = (tx*sx, ty*sy,
        // tz*sz, 1).
        let s = CATransform3D::make_scale(2.0, 3.0, 4.0);
        let translated = s.translate(1.0, 2.0, 3.0);
        assert!((translated.m41 - 2.0).abs() < 1e-5);
        assert!((translated.m42 - 6.0).abs() < 1e-5);
        assert!((translated.m43 - 12.0).abs() < 1e-5);
    }

    #[test]
    fn invert_round_trips() {
        let t = CATransform3D::make_translation(3.0, -4.0, 5.0)
            .scale(2.0, 0.5, -1.0)
            .rotate(std::f32::consts::FRAC_PI_3, 0.0, 0.0, 1.0);
        let inv = t.invert();
        let product = t.concat(inv);
        assert!(approx_eq(product, CATransform3DIdentity, 1e-4));
    }

    #[test]
    fn singular_invert_returns_input() {
        let mut singular = CATransform3DIdentity;
        singular.m11 = 0.0;
        singular.m22 = 0.0;
        singular.m33 = 0.0;
        singular.m44 = 0.0;
        assert!(approx_eq(singular.invert(), singular, 0.0));
    }

    #[test]
    fn affine_round_trips() {
        let affine = CGAffineTransform {
            a: 1.0,
            b: 2.0,
            c: 3.0,
            d: 4.0,
            tx: 5.0,
            ty: 6.0,
        };
        let lifted = CATransform3D::from_affine(affine);
        assert!(lifted.is_affine());
        let projected = lifted.to_affine();
        let same = projected.a == affine.a
            && projected.b == affine.b
            && projected.c == affine.c
            && projected.d == affine.d
            && projected.tx == affine.tx
            && projected.ty == affine.ty;
        assert!(same);
    }
}
