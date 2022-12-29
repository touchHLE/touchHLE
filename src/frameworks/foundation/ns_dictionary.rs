//! The `NSDictionary` class cluster, including `NSMutableDictionary`.

use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDictionary: NSObject

+ (id)dictionaryWithObjectsAndKeys:(id)first_object, ...va_args {
    let mut values = vec![first_object];
    loop {
        let object_or_key: id = va_args.next(env);
        if object_or_key == nil {
            break;
        }
        values.push(object_or_key);
    }
    let ns_number = crate::objc::msg_class![env; NSNumber class];
    for &value in &values {
        let class = env.mem.read(value.cast::<crate::mem::ConstPtr<id>>());
        if class == ns_number {
            println!("NSNumber");
        } else {
            println!("{:?}", super::ns_string::to_rust_string(env, value));
        }
    }
    unimplemented!("Construct dictionary with {:?}", values);
}

// TODO

@end

};
