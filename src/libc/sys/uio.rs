/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/uio.h`

use crate::dyld::FunctionExports;
use crate::libc::errno::{set_errno, EINVAL};
use crate::libc::posix_io::{write, FileDescriptor};
use crate::mem::{ConstPtr, GuestISize, GuestUSize, MutVoidPtr, SafeRead};
use crate::{export_c_func, Environment};

#[repr(C, packed)]
struct iovec {
    iov_base: MutVoidPtr,
    iov_len: GuestUSize,
}
unsafe impl SafeRead for iovec {}

fn writev(
    env: &mut Environment,
    fd: FileDescriptor,
    iov: ConstPtr<iovec>,
    iov_cnt: i32,
) -> GuestISize {
    if iov_cnt <= 0 {
        set_errno(env, EINVAL);
        return -1;
    }
    let mut res: GuestISize = 0;
    for i in 0..iov_cnt {
        let next = env.mem.read(iov + i.try_into().unwrap());
        let len = next.iov_len;
        let written = write(env, fd, next.iov_base.cast_const(), len);
        assert!(written >= 0); // TODO
        assert_eq!(written, len.try_into().unwrap()); // TODO
        res += written;
    }
    res
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(writev(_, _, _))];
