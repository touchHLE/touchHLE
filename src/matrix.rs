/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Transformation matrices implementation.
//!
//! This is used for, among other things:
//! - Generating transformations for OpenGL ES
//! - Translating between co-ordinate spaces for user inputs and display outputs
//!   (e.g. when the screen is rotated)
//! - Implementing transformation matrix APIs (e.g. `CGAffineTransform`)
//! - Interoperability between the above

/// OpenGL-style column-major matrix.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Matrix<const N: usize>([[f32; N]; N]);

impl<const N: usize> Matrix<N> {
    pub fn identity() -> Self {
        let mut matrix = [[0f32; N]; N];
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            matrix[i][i] = 1f32;
        }
        Matrix(matrix)
    }

    pub fn columns(&self) -> &[[f32; N]; N] {
        &self.0
    }

    // This constructor is used instead of direct field access so there can be
    // some flexibility with adjusting the representation.
    pub fn from_columns(columns: [[f32; N]; N]) -> Self {
        Matrix(columns)
    }

    pub fn from<const M: usize>(other: &Matrix<M>) -> Self {
        // FIXME: This is probably wrong for homogenous co-ordinates.
        let mut new = Self::identity();
        for i in 0..M {
            for j in 0..M {
                new.0[i][j] = other.0[i][j];
            }
        }
        new
    }

    pub fn multiply(&self, other: &Self) -> Self {
        let mut res = [[0f32; N]; N];
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            for j in 0..N {
                for k in 0..N {
                    res[i][j] += self.0[i][k] * other.0[k][j];
                }
            }
        }
        Matrix(res)
    }

    #[allow(dead_code)]
    pub fn transpose(&self) -> Self {
        let mut res = [[0f32; N]; N];
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            for j in 0..N {
                res[j][i] = self.0[i][j];
            }
        }
        Matrix(res)
    }

    /// Transform a column vector using the matrix: computes M × v where M is
    /// the matrix and v is `vector` as a column vector.
    pub fn transform(&self, vector: [f32; N]) -> [f32; N] {
        let mut new = [0f32; N];
        for (i, basis_vector) in self.columns().iter().enumerate() {
            for j in 0..N {
                new[j] += basis_vector[j] * vector[i];
            }
        }
        new
    }
}

impl Matrix<2> {
    pub fn determinant(&self) -> f32 {
        // https://en.wikipedia.org/wiki/Determinant
        let &Matrix([[a, c], [b, d]]) = self;
        a * d - b * c
    }

    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        // Square matrix is only invertible if its determinant is nonzero
        if det == 0.0 {
            return None;
        }
        // https://en.wikipedia.org/wiki/Invertible_matrix#Inversion_of_2_%C3%97_2_matrices
        let &Matrix([[a, c], [b, d]]) = self;
        Some(Matrix([
            [1.0 / det * d, 1.0 / det * -c],
            [1.0 / det * -b, 1.0 / det * a],
        ]))
    }

    pub fn y_flip() -> Matrix<2> {
        Matrix([[1.0, 0.0], [0.0, -1.0]])
    }

    pub fn z_rotation(angle: f32) -> Matrix<2> {
        Matrix([[angle.cos(), angle.sin()], [-angle.sin(), angle.cos()]])
    }

    pub fn scale_2d(x: f32, y: f32) -> Matrix<2> {
        Matrix([[x, 0.0], [0.0, y]])
    }
}
impl Matrix<3> {
    pub fn determinant(&self) -> f32 {
        // https://en.wikipedia.org/wiki/Determinant#Leibniz_formula
        let &Matrix([[a, d, g], [b, e, h], [c, f, i]]) = self;
        a * e * i + b * f * g + c * d * h - c * e * g - b * d * i - a * f * h
    }

    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        // Square matrix is only invertible if its determinant is nonzero
        if det == 0.0 {
            return None;
        }
        // https://en.wikipedia.org/wiki/Invertible_matrix#Inversion_of_3_%C3%97_3_matrices
        let &Matrix([[a, d, g], [b, e, h], [c, f, i]]) = self;
        let a_ = e * i - f * h;
        let b_ = -(d * i - f * g);
        let c_ = d * h - e * g;
        let d_ = -(b * i - c * h);
        let e_ = a * i - c * g;
        let f_ = -(a * h - b * g);
        let g_ = b * f - c * e;
        let h_ = -(a * f - c * d);
        let i_ = a * e - b * d;
        Some(Matrix([
            [1.0 / det * a_, 1.0 / det * b_, 1.0 / det * c_],
            [1.0 / det * d_, 1.0 / det * e_, 1.0 / det * f_],
            [1.0 / det * g_, 1.0 / det * h_, 1.0 / det * i_],
        ]))
    }

    pub fn x_rotation(angle: f32) -> Matrix<3> {
        Matrix([
            [1.0, 0.0, 0.0],
            [0.0, angle.cos(), angle.sin()],
            [0.0, -angle.sin(), angle.cos()],
        ])
    }
    pub fn y_rotation(angle: f32) -> Matrix<3> {
        Matrix([
            [angle.cos(), 0.0, -angle.sin()],
            [0.0, 1.0, 0.0],
            [angle.sin(), 0.0, angle.cos()],
        ])
    }

    pub fn translate_2d(x: f32, y: f32) -> Matrix<3> {
        Matrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [x, y, 1.0]])
    }
}
impl Matrix<4> {
    pub fn translate_3d(x: f32, y: f32, z: f32) -> Matrix<4> {
        Matrix([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, z, 1.0],
        ])
    }

    /// Determinant of a 4×4 matrix, computed via cofactor expansion along the
    /// first row.
    pub fn determinant(&self) -> f32 {
        let m = self.flat();
        let inv0 = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
            + m[9] * m[7] * m[14]
            + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        let inv4 = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
            - m[8] * m[7] * m[14]
            - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        let inv8 = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
            + m[8] * m[7] * m[13]
            + m[12] * m[5] * m[11]
            - m[12] * m[7] * m[9];
        let inv12 = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
            - m[8] * m[6] * m[13]
            - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        m[0] * inv0 + m[1] * inv4 + m[2] * inv8 + m[3] * inv12
    }

    /// Invert a 4×4 matrix, returning `None` if the matrix is singular.
    ///
    /// Uses the standard adjugate/cofactor algorithm (the same formula used by
    /// MESA's `gluInvertMatrix`) on the column-major flat representation.
    pub fn inverse(&self) -> Option<Self> {
        let m = self.flat();
        let mut inv = [0f32; 16];

        inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
            + m[9] * m[7] * m[14]
            + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
            - m[8] * m[7] * m[14]
            - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
            + m[8] * m[7] * m[13]
            + m[12] * m[5] * m[11]
            - m[12] * m[7] * m[9];
        inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
            - m[8] * m[6] * m[13]
            - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
            - m[9] * m[3] * m[14]
            - m[13] * m[2] * m[11]
            + m[13] * m[3] * m[10];
        inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
            + m[8] * m[3] * m[14]
            + m[12] * m[2] * m[11]
            - m[12] * m[3] * m[10];
        inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
            - m[8] * m[3] * m[13]
            - m[12] * m[1] * m[11]
            + m[12] * m[3] * m[9];
        inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
            + m[8] * m[2] * m[13]
            + m[12] * m[1] * m[10]
            - m[12] * m[2] * m[9];
        inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
            + m[5] * m[3] * m[14]
            + m[13] * m[2] * m[7]
            - m[13] * m[3] * m[6];
        inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
            - m[4] * m[3] * m[14]
            - m[12] * m[2] * m[7]
            + m[12] * m[3] * m[6];
        inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
            + m[4] * m[3] * m[13]
            + m[12] * m[1] * m[7]
            - m[12] * m[3] * m[5];
        inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
            - m[4] * m[2] * m[13]
            - m[12] * m[1] * m[6]
            + m[12] * m[2] * m[5];
        inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
            - m[5] * m[3] * m[10]
            - m[9] * m[2] * m[7]
            + m[9] * m[3] * m[6];
        inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
            + m[4] * m[3] * m[10]
            + m[8] * m[2] * m[7]
            - m[8] * m[3] * m[6];
        inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
            - m[4] * m[3] * m[9]
            - m[8] * m[1] * m[7]
            + m[8] * m[3] * m[5];
        inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
            + m[4] * m[2] * m[9]
            + m[8] * m[1] * m[6]
            - m[8] * m[2] * m[5];

        let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let inv_det = 1.0 / det;
        for v in &mut inv {
            *v *= inv_det;
        }

        let mut cols = [[0f32; 4]; 4];
        for c in 0..4 {
            for r in 0..4 {
                cols[c][r] = inv[c * 4 + r];
            }
        }
        Some(Matrix(cols))
    }

    /// Flatten the column-major storage to a length-16 array such that
    /// `flat[c*4 + r]` is the element at column `c`, row `r`.
    fn flat(&self) -> [f32; 16] {
        let mut out = [0f32; 16];
        for c in 0..4 {
            for r in 0..4 {
                out[c * 4 + r] = self.0[c][r];
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Matrix;

    fn approx_eq(a: &Matrix<4>, b: &Matrix<4>, eps: f32) -> bool {
        let ca = a.columns();
        let cb = b.columns();
        for c in 0..4 {
            for r in 0..4 {
                if (ca[c][r] - cb[c][r]).abs() > eps {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn matrix4_identity_inverse_is_identity() {
        let id = Matrix::<4>::identity();
        assert!(approx_eq(&id.inverse().unwrap(), &id, 1e-6));
    }

    #[test]
    fn matrix4_inverse_round_trip() {
        // A non-trivial invertible matrix: scale * rotation-like * translation.
        let m = Matrix::<4>::from_columns([
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 0.5, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [3.0, 4.0, 5.0, 1.0],
        ]);
        let inv = m.inverse().expect("matrix should be invertible");
        let id = Matrix::<4>::identity();
        assert!(approx_eq(&m.multiply(&inv), &id, 1e-5));
        assert!(approx_eq(&inv.multiply(&m), &id, 1e-5));
    }

    #[test]
    fn matrix4_singular_returns_none() {
        let m = Matrix::<4>::from_columns([
            [1.0, 2.0, 3.0, 4.0],
            [2.0, 4.0, 6.0, 8.0], // second column is 2× the first → singular
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]);
        assert!(m.inverse().is_none());
    }

    #[test]
    fn matrix4_determinant_diagonal() {
        let m = Matrix::<4>::from_columns([
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 5.0],
        ]);
        assert!((m.determinant() - 120.0).abs() < 1e-5);
    }
}
