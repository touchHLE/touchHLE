/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `netdb.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::Environment;

// TODO: struct definition
#[allow(non_camel_case_types)]
struct hostent {}

fn gethostbyname(env: &mut Environment, name: ConstPtr<u8>) -> MutPtr<hostent> {
    log!(
        "TODO: gethostbyname({:?} \"{}\") => NULL",
        name,
        env.mem.cstr_at_utf8(name).unwrap()
    );
    // TODO: set h_errno
    Ptr::null()
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(gethostbyname(_))];
