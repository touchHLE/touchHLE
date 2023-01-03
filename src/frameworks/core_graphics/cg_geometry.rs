//! `CGGeometry.h` (`CGPoint`, `CGSize`, `CGRect`, etc)

use super::CGFloat;
use crate::abi::impl_GuestRet_for_large_struct;
use crate::mem::SafeRead;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}
unsafe impl SafeRead for CGPoint {}
impl_GuestRet_for_large_struct!(CGPoint);

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}
unsafe impl SafeRead for CGSize {}
impl_GuestRet_for_large_struct!(CGSize);

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}
unsafe impl SafeRead for CGRect {}
impl_GuestRet_for_large_struct!(CGRect);
