//! Handling of Objective-C messaging (`objc_msgSend` and friends).
//!
//! Resources:
//! - Apple's [Objective-C Runtime Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtHowMessagingWorks.html)
//! - [Apple's documentation of `objc_msgSend`](https://developer.apple.com/documentation/objectivec/1456712-objc_msgsend)
//! - Mike Ash's [objc_msgSend's New Prototype](https://www.mikeash.com/pyblog/objc_msgsends-new-prototype.html)

use super::{id, nil, ObjC, IMP, SEL};
use crate::abi::CallFromHost;
use crate::Environment;

/// `objc_msgSend` itself, the main function of Objective-C.
///
/// Note that while only the receiver and selector parameters are declared here,
/// a call to `objc_msgSend` may have additional arguments to be forwarded (or
/// rather, left untouched) by `objc_msgSend` when it tail-calls the method
/// implementation it looks up. This is invisible to the Rust type system; we're
/// relying on [crate::abi::CallFromGuest] here.
///
/// Similarly, the return value of `objc_msgSend` is whatever value is returned
/// by the method implementation. We are relying on CallFromGuest not
/// overwriting it.
#[allow(non_snake_case)]
pub fn objc_msgSend(env: &mut Environment, receiver: id, selector: SEL) {
    if receiver == nil {
        unimplemented!()
    } // TODO: nil handling

    let orig_class = ObjC::read_isa(receiver, &env.mem);
    assert!(orig_class != nil);

    // Traverse the chain of superclasses to find the method implementation.

    let mut class = orig_class;
    loop {
        if class == nil {
            assert!(class != orig_class);

            let class_host_object = env.objc.get_host_object(orig_class).unwrap();
            let &super::ClassHostObject {
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

        let host_object = env.objc.get_host_object(class).unwrap();

        if let Some(&super::ClassHostObject {
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
        } else if let Some(&super::UnimplementedClass {
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

/// Wrapper around [objc_msgSend] which, together with [msg], makes it easy to
/// send messages in host code. Warning: all types are inferred from the
/// call-site, be very sure you get them correct!
///
/// TODO: Ideally we can constrain the first two args to be `id` and `SEL`?
///
/// TODO: Could we pass along dynamic type information to `objc_msgSend` so it
/// can do runtime type-checking? Perhaps only in debug builds.
pub fn msg_send<R, P>(env: &mut Environment, args: P) -> R
where
    fn(&mut Environment, id, SEL): CallFromHost<R, P>,
{
    (objc_msgSend as fn(&mut Environment, id, SEL)).call_from_host(env, args)
}

/// Macro for sending a message which imitates the Objective-C messaging syntax.
/// See [msg_send] for the underlying implementation. Warning: all types are
/// inferred from the call-site, be very sure you get them correct!
///
/// ```rust
/// msg![env; foo setBar:bar withQux:qux];
/// ```
///
/// desugars to:
///
/// ```rust
/// {
///     let sel = env.objc.lookup_selector("setFoo:withBar").unwrap();
///     msg_send(env, (foo, sel, bar, qux))
/// }
/// ```
///
/// Note that argument values that aren't a bare single identifier like `foo`
/// need to be bracketed.
///
/// See also [msg_class], if you want to send a message to a class.
#[macro_export]
macro_rules! msg {
    [$env:expr; $receiver:tt $name:ident $(: $arg1:tt)?
                             $($namen:ident: $argn:tt)*] => {
        {
            let sel = $crate::objc::selector!($($arg1;)? $name $(, $namen)*);
            let sel = $env.objc.lookup_selector(sel).unwrap();
            let args = ($receiver, sel, $($arg1,)? $($argn),*);
            $crate::objc::msg_send($env, args)
        }
    }
}
pub use crate::msg; // #[macro_export] is weird...

/// Variant of [msg] for sending a message to a named class. Useful for calling
/// class methods, especially `new`.
///
/// ```rust
/// msg_class![env; SomeClass alloc]
/// ```
///
/// desugars to:
///
/// ```rust
/// msg![env; (env.objc.get_known_class("SomeClass"), &mut env.mem) alloc]
/// ```
#[macro_export]
macro_rules! msg_class {
    [$env:expr; $receiver_class:ident $name:ident $(: $arg1:tt)?
                                      $($namen:ident: $argn:tt)*] => {
        {
            let class = $env.objc.get_known_class(
                stringify!($receiver_class),
                &mut $env.mem
            );
            $crate::objc::msg![$env; class $name $(: $arg1)?
                                           $($namen: $argn)*]
        }
    }
}
pub use crate::msg_class; // #[macro_export] is weird...
