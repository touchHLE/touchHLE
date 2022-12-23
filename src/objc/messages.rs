//! Handling of Objective-C messaging (`objc_msgSend` and friends).
//!
//! Resources:
//! - Apple's [Objective-C Runtime Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtHowMessagingWorks.html)
//! - [Apple's documentation of `objc_msgSend`](https://developer.apple.com/documentation/objectivec/1456712-objc_msgsend)
//! - Mike Ash's [objc_msgSend's New Prototype](https://www.mikeash.com/pyblog/objc_msgsends-new-prototype.html)

use super::{id, nil, IMP, SEL};
use crate::Environment;

/// `objc_msgSend` itself, the main function of Objective-C.
///
/// Note that while only the receiver and selector parameters are declared here,
/// a call to `objc_msgSend` may have additional arguments to be forwarded (or
/// rather, left untouched) by `objc_msgSend` when it tail-calls the method
/// implementation it looks up. This is invisible to the Rust type system; we're
/// relying on [crate::abi::CallFromGuest] here.
#[allow(non_snake_case)]
pub fn objc_msgSend(env: &mut Environment, receiver: id, selector: SEL) {
    if receiver == nil {
        unimplemented!()
    } // TODO: nil handling

    let orig_class = super::ObjC::read_isa(receiver, &env.mem);
    assert!(orig_class != nil);

    // Traverse the chain of superclasses to find the method implementation.

    let mut class = orig_class;
    loop {
        if class == nil {
            assert!(class != orig_class);

            let class_host_object = env.objc.objects.get(&orig_class).unwrap();
            let &super::classes::ClassHostObject {
                ref name,
                is_metaclass,
                ..
            } = class_host_object.as_any().downcast_ref().unwrap();

            panic!(
                "{} {:?} ({}class \"{}\", {:?}) does not respond to selector \"{}\"!",
                if is_metaclass { "Class" } else { "Object" },
                receiver,
                if is_metaclass { "meta" } else { "" },
                name,
                orig_class,
                selector.as_str(&env.mem),
            );
        }

        let host_object = env.objc.objects.get(&class).unwrap();

        if let Some(&super::classes::ClassHostObject {
            superclass,
            ref methods,
            ..
        }) = host_object.as_any().downcast_ref()
        {
            if let Some(imp) = methods.get(&selector) {
                let IMP::Host(host_imp) = imp;
                host_imp.call_from_guest(env);
                return;
            } else {
                class = superclass;
            }
        } else if let Some(&super::classes::UnimplementedClass {
            ref name,
            is_metaclass,
        }) = host_object.as_any().downcast_ref()
        {
            panic!(
                "Class \"{}\" ({:?}) is unimplemented. Call to {} method \"{}\".",
                name,
                class,
                if is_metaclass { "class" } else { "instance" },
                selector.as_str(&env.mem),
            );
        } else {
            panic!(
                "Item {:?} in superclass chain of object {:?}'s class {:?} has an unexpected host object type.",
                class, receiver, orig_class
            );
        }
    }
}
