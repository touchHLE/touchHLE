/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::Environment;
use crate::mem::MutVoidPtr;
use crate::dyld::{ConstantExports, export_c_func, FunctionExports, HostConstant};

fn NSSetUncaughtExceptionHandler(
    _: &mut Environment,
    handler: MutVoidPtr, // void (NSException *)()
) {
    log!("TODO: NSSetUncaughtExceptionHandler({:?})", handler);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSSetUncaughtExceptionHandler(_))];

pub const CONSTANTS: ConstantExports = &[
    (
        "_NSInvalidArgumentException",
        HostConstant::NSString("NSInvalidArgumentException"),
    ),
];