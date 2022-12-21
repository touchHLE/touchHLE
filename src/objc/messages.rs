//! Handling of Objective-C messaging (`objc_msgSend` and friends).
//!
//! Resources:
//! - Apple's [Objective-C Runtime Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtHowMessagingWorks.html)
//! - [Apple's documentation of `objc_msgSend`](https://developer.apple.com/documentation/objectivec/1456712-objc_msgsend)
//! - Mike Ash's [objc_msgSend's New Prototype](https://www.mikeash.com/pyblog/objc_msgsends-new-prototype.html)

use super::{id, SEL};
use crate::Environment;

#[allow(non_snake_case)]
pub fn objc_msgSend(
    env: &mut Environment,
    receiver: id,
    selector: SEL,
    // other arguments not handled yet
) {
    let class = super::ObjC::read_isa(receiver, &env.mem);

    let host_object = env.objc.objects.get(&class).unwrap(); // TODO error message

    if let Some(&super::classes::UnimplementedClass {
        ref name,
        is_metaclass,
    }) = host_object.as_any().downcast_ref()
    {
        panic!(
            "Call to {} method \"{}\" of unimplemented class \"{}\" ({:?})",
            if is_metaclass { "class" } else { "instance" },
            selector.as_str(&env.mem),
            name,
            class,
        );
    }

    unimplemented!(
        "objc_msgSend({:?}, {:?} = {:?}, ...)",
        receiver,
        selector,
        selector.as_str(&env.mem),
    );
}
