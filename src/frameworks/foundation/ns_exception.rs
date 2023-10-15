/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutVoidPtr;
use crate::Environment;

fn NSSetUncaughtExceptionHandler(
    _: &mut Environment,
    handler: MutVoidPtr, // void (NSException *)()
) {
    log!("TODO: NSSetUncaughtExceptionHandler({:?})", handler);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSSetUncaughtExceptionHandler(_))];
