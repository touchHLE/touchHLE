//! `NSObject`, the root of most class hierarchies in Objective-C.

use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, objc_classes, ClassExports, TrivialHostObject};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSObject

+ (id)alloc {
    msg![env; this allocWithZone:(MutVoidPtr::null())]
}
+ (id)allocWithZone:(MutVoidPtr)_zone { // struct _NSZone*
    env.objc.alloc_object(this, Box::new(TrivialHostObject), &mut env.mem)
}

- (id)init {
    this
}

@end

};
