use crate::abi::GuestArg;
use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::environment::Environment;
use crate::frameworks::core_graphics::CGFloat;
use crate::mem::SafeRead;
use crate::objc::HostObject;
use crate::{export_c_func, impl_GuestRet_for_large_struct};
use std::ops::{Index, IndexMut};

#[derive(Debug, Default, Copy, Clone)]
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

impl Index<(usize, usize)> for CATransform3D {
    type Output = CGFloat;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        match index {
            (1, 1) => &self.m11,
            (1, 2) => &self.m12,
            (1, 3) => &self.m13,
            (1, 4) => &self.m14,
            (2, 1) => &self.m21,
            (2, 2) => &self.m22,
            (2, 3) => &self.m23,
            (2, 4) => &self.m24,
            (3, 1) => &self.m31,
            (3, 2) => &self.m32,
            (3, 3) => &self.m33,
            (3, 4) => &self.m34,
            (4, 1) => &self.m41,
            (4, 2) => &self.m42,
            (4, 3) => &self.m43,
            (4, 4) => &self.m44,
            _ => panic!("Invalid index")
        }
    }
}
impl IndexMut<(usize, usize)> for CATransform3D {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        match index {
            (1, 1) => &mut self.m11,
            (1, 2) => &mut self.m12,
            (1, 3) => &mut self.m13,
            (1, 4) => &mut self.m14,
            (2, 1) => &mut self.m21,
            (2, 2) => &mut self.m22,
            (2, 3) => &mut self.m23,
            (2, 4) => &mut self.m24,
            (3, 1) => &mut self.m31,
            (3, 2) => &mut self.m32,
            (3, 3) => &mut self.m33,
            (3, 4) => &mut self.m34,
            (4, 1) => &mut self.m41,
            (4, 2) => &mut self.m42,
            (4, 3) => &mut self.m43,
            (4, 4) => &mut self.m44,
            _ => panic!("Invalid index")
        }
    }
}
impl CATransform3D {
    pub fn identity() -> Self {
        let mut transform = CATransform3D::default();
        transform[(1,1)] = 1.0;
        transform[(2,2)] = 1.0;
        transform[(3,3)] = 1.0;
        transform[(4,4)] = 1.0;
        transform
    }
}
unsafe impl SafeRead for CATransform3D {}
impl HostObject for CATransform3D {}
impl_GuestRet_for_large_struct!(CATransform3D);
impl GuestArg for CATransform3D {
    // TODO figure out how to pass big structs
    //      as arguments in the ARMv6 calling convention
    const REG_COUNT: usize = 0;

    fn from_regs(_regs: &[u32]) -> Self {
        CATransform3D::identity()
    }

    fn to_regs(self, _regs: &mut [u32]) {
    }
}
pub fn CATransform3DMakeScale(_env: &mut Environment, sx: CGFloat, sy: CGFloat, sz: CGFloat) -> CATransform3D {
    let mut transform = CATransform3D::default();
    set_scale(&mut transform, sx, sy, sz);
    transform
}

pub fn set_scale(transform: &mut CATransform3D, sx: CGFloat, sy: CGFloat, sz: CGFloat) {
    transform[(1,1)] = sx;
    transform[(2,2)] = sy;
    transform[(3,3)] = sz;
    transform[(4,4)] = 1.0;
}

pub const x_translation_index: (usize, usize) = (1, 4);
pub const y_translation_index: (usize, usize) = (2, 4);
pub const z_translation_index: (usize, usize) = (3, 4);

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CATransform3DMakeScale(_,_,_))
];


pub const CONSTANTS: ConstantExports = &[(
    "_CATransform3DIdentity",
    HostConstant::Custom(|env| {
        env.mem.alloc_and_write(CATransform3D::identity())
            .cast()
            .cast_const()
    }),
)];