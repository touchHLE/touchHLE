//! `NSProcessInfo`.

use super::NSTimeInterval;
use crate::objc::{objc_classes, ClassExports};
use std::time::Instant;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSProcessInfo: NSObject

+ (NSTimeInterval)systemUptime {
    Instant::now().duration_since(env.startup_time).as_secs_f64()
}

@end

};
