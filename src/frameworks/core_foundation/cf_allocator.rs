//! `CFAllocator`. Currently there is no actual support for multiple allocators.

use super::CFTypeRef;
use crate::dyld::{ConstantExports, HostConstant};
use crate::mem::Ptr;

#[allow(dead_code)]
pub type CFAllocatorRef = CFTypeRef;

#[allow(dead_code)]
pub const kCFAllocatorDefault: CFAllocatorRef = Ptr::null();

pub const CONSTANTS: ConstantExports = &[("_kCFAllocatorDefault", HostConstant::NullPtr)];
