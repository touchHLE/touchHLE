/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Enough of a matrix implementation to handle simple 2D rotations.

/// Column-major.
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

    pub fn from<const M: usize>(other: &Matrix<M>) -> Self {
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

    /// Transform a vector using the matrix.
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
    pub fn y_flip() -> Matrix<2> {
        Matrix([[1.0, 0.0], [0.0, -1.0]])
    }

    pub fn z_rotation(angle: f32) -> Matrix<2> {
        Matrix([[angle.cos(), angle.sin()], [-angle.sin(), angle.cos()]])
    }
}
impl Matrix<3> {
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
}
