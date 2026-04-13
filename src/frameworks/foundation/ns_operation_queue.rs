/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSOperation` and `NSOperationQueue` stubs.
use crate::objc::{id, nil, objc_classes, ClassExports};
use crate::msg_class;
use crate::msg;

pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);

@implementation NSOperation: NSObject

- (id)init {
    this
}

- (())start {
}

- (())cancel {
}

- (())main {
}

- (bool)isFinished {
    false
}

- (bool)isCancelled {
    false
}

- (bool)isExecuting {
    false
}

- (bool)isReady {
    true
}

@end

@implementation NSOperationQueue: NSObject

+ (id)mainQueue {
    let queue = msg_class![env; NSOperationQueue alloc];
    msg![env; queue init]
}

+ (id)currentQueue {
    let queue = msg_class![env; NSOperationQueue alloc];
    msg![env; queue init]
}

- (id)init {
    this
}

- (())addOperation:(id)_op {
}

- (())addOperations:(id)_ops waitUntilFinished:(bool)_wait {
}

- (())setMaxConcurrentOperationCount:(i32)_count {
}

- (())setSuspended:(bool)_suspended {
}

- (())cancelAllOperations {
}

- (())waitUntilAllOperationsAreFinished {
}

@end
};