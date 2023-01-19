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
}

impl Matrix<2> {
    pub fn z_rotation(angle: f32) -> Matrix<2> {
        Matrix([[angle.cos(), angle.sin()], [-angle.sin(), angle.cos()]])
    }
}
