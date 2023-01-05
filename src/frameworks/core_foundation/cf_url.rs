//! `CFURL`.
//!
//! This is toll-free bridged to `CFURL` in Apple's implementation. Here it is
//! the same type.

use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::MutPtr;
use crate::objc::msg;
use crate::Environment;

pub type CFURLRef = super::CFTypeRef;

pub fn CFURLGetFileSystemRepresentation(
    env: &mut Environment,
    url: CFURLRef,
    resolve_against_base: bool,
    buffer: MutPtr<u8>,
    buffer_size: CFIndex,
) -> bool {
    assert!(!resolve_against_base); // unimplemented
    let buffer_size: NSUInteger = buffer_size.try_into().unwrap();

    msg![env; url getFileSystemRepresentation:buffer
                                    maxLength:buffer_size]
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(CFURLGetFileSystemRepresentation(_, _, _, _))];
