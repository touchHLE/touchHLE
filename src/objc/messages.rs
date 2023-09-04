/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C messaging (`objc_msgSend` and friends).
//!
//! Resources:
//! - Apple's [Objective-C Runtime Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtHowMessagingWorks.html)
//! - [Apple's documentation of `objc_msgSend`](https://developer.apple.com/documentation/objectivec/1456712-objc_msgsend)
//! - Mike Ash's [objc_msgSend's New Prototype](https://www.mikeash.com/pyblog/objc_msgsends-new-prototype.html)
//! - Peter Steinberger's [Calling Super at Runtime in Swift](https://steipete.com/posts/calling-super-at-runtime/) explains `objc_msgSendSuper2`

use std::any::{Any, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::{id, nil, Class, ObjC, IMP, SEL};
use crate::abi::{CallFromHost, GuestRet};
use crate::mem::{ConstPtr, MutVoidPtr, SafeRead};
use crate::Environment;

/// The core implementation of `objc_msgSend`, the main function of Objective-C.
///
/// Note that while only two parameters (usually receiver and selector) are
/// defined by the wrappers over this function, a call to an `objc_msgSend`
/// variant may have additional arguments to be forwarded (or rather, left
/// untouched) by `objc_msgSend` when it tail-calls the method implementation it
/// looks up. This is invisible to the Rust type system; we're relying on
/// [crate::abi::CallFromGuest] here. To provide (limited) typechecking support,
/// messages that are called by host functions and received by host classes can have
/// their parameters checked at runtime, which is done transparently through the [msg]
/// macro (and friends).
///
/// Similarly, the return value of `objc_msgSend` is whatever value is returned
/// by the method implementation. We are relying on CallFromGuest not
/// overwriting it.
#[allow(non_snake_case)]
fn objc_msgSend_inner(
    env: &mut Environment,
    receiver: id,
    selector: SEL,
    super2: Option<Class>,
    type_id: Option<u64>,
) {
    if receiver == nil {
        // https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjectiveC/Chapters/ocObjectsClasses.html#//apple_ref/doc/uid/TP30001163-CH11-SW7
        log_dbg!("[nil {}]", selector.as_str(&env.mem));
        env.cpu.regs_mut()[0..2].fill(0);
        return;
    }

    let orig_class = super2.unwrap_or_else(|| ObjC::read_isa(receiver, &env.mem));
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
                "{} {:?} ({}class \"{}\", {:?}){} does not respond to selector \"{}\"!",
                if is_metaclass { "Class" } else { "Object" },
                receiver,
                if is_metaclass { "meta" } else { "" },
                name,
                orig_class,
                if super2.is_some() {
                    "'s superclass"
                } else {
                    ""
                },
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
            // Skip method lookup on first iteration if this is the super-call
            // variant of objc_msgSend (look up the superclass first)
            if super2.is_some() && class == orig_class {
                class = superclass;
                continue;
            }

            if let Some(imp) = methods.get(&selector) {
                match imp {
                    IMP::Host(host_imp) => {
                        debug_assert!(type_id.map_or(true, |tid| tid == host_imp.args_type_id()));
                        host_imp.call_from_guest(env)
                    }
                    // We can't create a new stack frame, because that would
                    // interfere with pass-through of stack arguments.
                    // TODO: method_t stores types for guest IMPs, so it should (theoretically) be
                    // possible to typecheck host -> guest calls.
                    IMP::Guest(guest_imp) => guest_imp.call_without_pushing_stack_frame(env),
                }
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
        } else if let Some(&super::FakeClass {
            ref name,
            is_metaclass,
        }) = host_object.as_any().downcast_ref()
        {
            log!(
                "Call to faked class \"{}\" ({:?}) {} method \"{}\". Behaving as if message was sent to nil.",
                name,
                class,
                if is_metaclass { "class" } else { "instance" },
                selector.as_str(&env.mem),
            );
            env.cpu.regs_mut()[0..2].fill(0);
            return;
        } else {
            panic!(
                "Item {:?} in superclass chain of object {:?}'s class {:?} has an unexpected host object type.",
                class, receiver, orig_class
            );
        }
    }
}

/// Standard variant of `objc_msgSend`. See [objc_msgSend_inner].
#[allow(non_snake_case)]
pub(super) fn objc_msgSend(env: &mut Environment, receiver: id, selector: SEL) {
    objc_msgSend_inner(env, receiver, selector, /* super2: */ None, None)
}

#[repr(C, packed)]
/// A pointer to this struct replaces the normal receiver parameter for
/// `objc_msgSend(stret)_debug` and [msg_send] when debug assertions are on.
pub struct objc_debug {
    pub receiver: id,
    /// Type info for arguments (not including the reciever and selector).
    pub type_id: u64,
}

unsafe impl SafeRead for objc_debug {}

/// Non-standard variant of `objc_msgSend` for host code that checks passed types
/// at runtime. See [objc_msgSend_inner].
///
/// The ABI here is similar to [objc_msgSendSuper2], with a struct passed in to
/// avoid disturbing the arguments to the message.
#[allow(non_snake_case)]
pub(super) fn objc_msgSend_debug(
    env: &mut Environment,
    debug_ptr: ConstPtr<objc_debug>,
    selector: SEL,
) {
    let objc_debug { receiver, type_id } = env.mem.read(debug_ptr);

    crate::abi::write_next_arg(&mut 0, env.cpu.regs_mut(), &mut env.mem, receiver);

    objc_msgSend_inner(
        env,
        receiver,
        selector,
        /* super2: */ None,
        Some(type_id),
    )
}

/// Variant of `objc_msgSend` for methods that return a struct via a pointer.
/// See [objc_msgSend_inner].
///
/// The first parameter here is the pointer for the struct return. This is an
/// ABI detail that is usually hidden and handled behind-the-scenes by
/// [crate::abi], but `objc_msgSend` is a special case because of the
/// pass-through behaviour. Of course, the pass-through only works if the [IMP]
/// also has the pointer parameter. The caller therefore has to pick the
/// appropriate `objc_msgSend` variant depending on the method it wants to call.
pub(super) fn objc_msgSend_stret(
    env: &mut Environment,
    _stret: MutVoidPtr,
    receiver: id,
    selector: SEL,
) {
    objc_msgSend_inner(env, receiver, selector, /* super2: */ None, None)
}

/// Variant of [objc_msgSend_stret] for host calls that checks passed types (at runtime).
/// See [objc_msgSend_debug] and [objc_msgSend_inner].
#[allow(non_snake_case)]
pub(super) fn objc_msgSend_stret_debug(
    env: &mut Environment,
    _stret: MutVoidPtr,
    debug_ptr: ConstPtr<objc_debug>,
    selector: SEL,
) {
    let objc_debug { receiver, type_id } = env.mem.read(debug_ptr);

    crate::abi::write_next_arg(&mut 1, env.cpu.regs_mut(), &mut env.mem, receiver);

    objc_msgSend_inner(
        env,
        receiver,
        selector,
        /* super2: */ None,
        Some(type_id),
    )
}

#[repr(C, packed)]
/// A pointer to this struct replaces the normal receiver parameter for
/// `objc_msgSendSuper2` and [msg_send_super2].
pub struct objc_super {
    pub receiver: id,
    /// If this is used with `objc_msgSendSuper` (not implemented here, TODO),
    /// this is a pointer to the superclass to look up the method on.
    /// If this is used with `objc_msgSendSuper2`, this is a pointer to a class
    /// and the superclass will be looked up from it.
    pub class: Class,
}
unsafe impl SafeRead for objc_super {}

/// Variant of `objc_msgSend` for supercalls. See [objc_msgSend_inner].
///
/// This variant has a weird ABI because it needs to receive an additional piece
/// of information (a class pointer), but it can't actually take this as an
/// extra parameter, because that would take one of the argument slots reserved
/// for arguments passed onto the method implementation. Hence the [objc_super]
/// pointer in place of the normal [id].
#[allow(non_snake_case)]
pub(super) fn objc_msgSendSuper2(
    env: &mut Environment,
    super_ptr: ConstPtr<objc_super>,
    selector: SEL,
) {
    let objc_super { receiver, class } = env.mem.read(super_ptr);

    // Rewrite first argument to match the normal ABI.
    crate::abi::write_next_arg(&mut 0, env.cpu.regs_mut(), &mut env.mem, receiver);

    objc_msgSend_inner(
        env,
        receiver,
        selector,
        /* super2: */ Some(class),
        None,
    )
}

#[repr(C, packed)]
/// Variant of [objc_super] that enables typechecking from host calls.
/// A pointer to this struct replaces the normal receiver parameter for
/// `objc_msgSendSuper2_debug` and [msg_send_super2_debug].
pub struct objc_super_debug {
    pub receiver: id,
    pub class: Class,
    pub typecheck_id: u64,
}
unsafe impl SafeRead for objc_super_debug {}

/// Variant of [objc_msgSendSuper2] for host calls that checks passed types (at runtime).
/// See [objc_msgSend_debug] and [objc_msgSend_inner].
#[allow(non_snake_case)]
pub(super) fn objc_msgSendSuper2_debug(
    env: &mut Environment,
    super_ptr: ConstPtr<objc_super_debug>,
    selector: SEL,
) {
    let objc_super_debug {
        receiver,
        class,
        typecheck_id,
    } = env.mem.read(super_ptr);

    // Rewrite first argument to match the normal ABI.
    crate::abi::write_next_arg(&mut 0, env.cpu.regs_mut(), &mut env.mem, receiver);

    objc_msgSend_inner(
        env,
        receiver,
        selector,
        /* super2: */ Some(class),
        Some(typecheck_id),
    )
}

/// Wrapper around [objc_msgSend] which, together with [msg], makes it easy to
/// send messages in host code. Warning: all types are inferred from the
/// call-site, be very sure you get them correct!
///
/// TODO: Ideally we can constrain the first two args to be `id` and `SEL`?
pub fn msg_send<R, P>(env: &mut Environment, args: P) -> R
where
    fn(&mut Environment, id, SEL): CallFromHost<R, P>,
    fn(&mut Environment, MutVoidPtr, id, SEL): CallFromHost<R, P>,
    R: GuestRet,
{
    if R::SIZE_IN_MEM.is_some() {
        (objc_msgSend_stret as fn(&mut Environment, MutVoidPtr, id, SEL)).call_from_host(env, args)
    } else {
        (objc_msgSend as fn(&mut Environment, id, SEL)).call_from_host(env, args)
    }
}

/// Debug variant of [msg_send] that passes type info to be checked. You probably want to
/// use [msg] rather than calling this directly.
pub fn msg_send_debug<R, P>(env: &mut Environment, args: P) -> R
where
    fn(&mut Environment, ConstPtr<objc_debug>, SEL): CallFromHost<R, P>,
    fn(&mut Environment, MutVoidPtr, ConstPtr<objc_debug>, SEL): CallFromHost<R, P>,
    R: GuestRet,
{
    if R::SIZE_IN_MEM.is_some() {
        (objc_msgSend_stret_debug as fn(&mut Environment, MutVoidPtr, ConstPtr<objc_debug>, SEL))
            .call_from_host(env, args)
    } else {
        (objc_msgSend_debug as fn(&mut Environment, ConstPtr<objc_debug>, SEL))
            .call_from_host(env, args)
    }
}

/// [msg_send] but for super-calls (calls [objc_msgSendSuper2]). You probably
/// want to use [msg_super] rather than calling this directly.
pub fn msg_send_super2<R, P>(env: &mut Environment, args: P) -> R
where
    fn(&mut Environment, ConstPtr<objc_super>, SEL): CallFromHost<R, P>,
    fn(&mut Environment, MutVoidPtr, ConstPtr<objc_super>, SEL): CallFromHost<R, P>,
    R: GuestRet,
{
    if R::SIZE_IN_MEM.is_some() {
        todo!() // no stret yet
    } else {
        (objc_msgSendSuper2 as fn(&mut Environment, ConstPtr<objc_super>, SEL))
            .call_from_host(env, args)
    }
}

/// Debug variant of [msg_send] that passes type info to be checked. You probably
/// want to use [msg_super] rather than calling this directly.
pub fn msg_send_super2_debug<R, P>(env: &mut Environment, args: P) -> R
where
    fn(&mut Environment, ConstPtr<objc_super_debug>, SEL): CallFromHost<R, P>,
    fn(&mut Environment, MutVoidPtr, ConstPtr<objc_super_debug>, SEL): CallFromHost<R, P>,
    R: GuestRet,
{
    if R::SIZE_IN_MEM.is_some() {
        todo!() // no stret yet
    } else {
        (objc_msgSendSuper2_debug as fn(&mut Environment, ConstPtr<objc_super_debug>, SEL))
            .call_from_host(env, args)
    }
}

/// Macro for sending a message which imitates the Objective-C messaging syntax.
/// See [msg_send] for the underlying implementation. Warning: Types are only
/// checked at runtime, when the message is actually called, and only in builds with
/// debug assertions enabled.
///
///
/// ```ignore
/// msg![env; foo setBar:bar withQux:qux];
/// ```
///
/// desugars to:
///
/// ```ignore
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
                             $($namen:ident: $argn:tt)* $(,$va_argn:ident)*] => {
        {
            let sel = $crate::objc::selector!($($arg1;)? $name $(, $namen)*);
            let sel = $env.objc.lookup_selector(sel)
                .expect("Unknown selector");
            if cfg!(debug_assertions)
            {
                let type_id = $crate::objc::generate_type_id(($($arg1.clone(),)? $($argn.clone(),)*));

                let sp = &mut $env.cpu.regs_mut()[$crate::cpu::Cpu::SP];
                let old_sp = *sp;
                *sp -= $crate::mem::guest_size_of::<$crate::objc::objc_debug>();
                let debug_ptr = $crate::mem::Ptr::from_bits(*sp);
                $env.mem.write(debug_ptr, $crate::objc::objc_debug {
                    receiver: $receiver,
                    type_id,
                });

                let args = (debug_ptr, sel, $($arg1,)? $($argn),*);
                let res = $crate::objc::msg_send_debug($env, args);

                $env.cpu.regs_mut()[$crate::cpu::Cpu::SP] = old_sp;

                res
            } else {
                let args = ($receiver, sel, $($arg1,)? $($argn),*);
                $crate::objc::msg_send($env, args)
            }
        }
    }
}
pub use crate::msg; // #[macro_export] is weird...

/// Variant of [msg] that does not perform typechecking.
///
/// You might need this if you're intentionally mistyping arguments, or if you
/// can't allocate on the stack.
#[macro_export]
macro_rules! msg_unchecked {
    [$env:expr; $receiver:tt $name:ident $(: $arg1:tt)?
                             $($namen:ident: $argn:tt)*] => {
        {
            let sel = $crate::objc::selector!($($arg1;)? $name $(, $namen)*);
            let sel = $env.objc.lookup_selector(sel)
                .expect("Unknown selector");
            let args = ($receiver, sel, $($arg1,)? $($argn),*);
            $crate::objc::msg_send($env, args)
        }
    }
}
pub use crate::msg_unchecked;

/// Variant of [msg] for super-calls.
///
/// Unlike the other variants, this macro can only be used within
/// [crate::objc::objc_classes], because it relies on that macro defining a
/// constant containing the name of the current class.
///
/// ```ignore
/// msg_super![env; this init]
/// ```
///
/// desugars to something like this, if the current class is `SomeClass`:
///
/// ```ignore
/// {
///     let super_arg_ptr = push_to_stack(env, objc_super {
///         receiver: this,
///         class: env.objc.get_known_class("SomeClass", &mut env.mem),
///     });
///     let sel = env.objc.lookup_selector("init").unwrap();
///     let res = msg_send_super2(env, (super_arg_ptr, sel));
///     pop_from_stack::<objc_super>(env);
///     res
/// }
/// ```
#[macro_export]
macro_rules! msg_super {
    [$env:expr; $receiver:tt $name:ident $(: $arg1:tt)?
                             $($namen:ident: $argn:tt)*] => {
        {
            let class = $env.objc.get_known_class(
                _OBJC_CURRENT_CLASS,
                &mut $env.mem
            );
            let sel = $crate::objc::selector!($($arg1;)? $name $(, $namen)*);
            let sel = $env.objc.lookup_selector(sel)
                .expect("Unknown selector");
            if cfg!(debug_assertions)
            {
                let sp = &mut $env.cpu.regs_mut()[$crate::cpu::Cpu::SP];
                let old_sp = *sp;
                *sp -= $crate::mem::guest_size_of::<$crate::objc::objc_super_debug>();
                let super_ptr = $crate::mem::Ptr::from_bits(*sp);
                let typecheck_id = $crate::objc::generate_type_id(($($arg1,)? $($argn,)*));
                $env.mem.write(super_ptr, $crate::objc::objc_super_debug {
                    receiver: $receiver,
                    class,
                    typecheck_id,
                });

                let args = (super_ptr, sel, $($arg1,)? $($argn),*);
                let res = $crate::objc::msg_send_super2_debug($env, args);

                $env.cpu.regs_mut()[$crate::cpu::Cpu::SP] = old_sp;

                res
            } else {
                let sp = &mut $env.cpu.regs_mut()[$crate::cpu::Cpu::SP];
                let old_sp = *sp;
                *sp -= $crate::mem::guest_size_of::<$crate::objc::objc_super>();
                let super_ptr = $crate::mem::Ptr::from_bits(*sp);
                $env.mem.write(super_ptr, $crate::objc::objc_super {
                    receiver: $receiver,
                    class,
                });

                let args = (super_ptr, sel, $($arg1,)? $($argn),*);
                let res = $crate::objc::msg_send_super2($env, args);

                $env.cpu.regs_mut()[$crate::cpu::Cpu::SP] = old_sp;

                res
            }
        }
    }
}
pub use crate::msg_super; // #[macro_export] is weird...

/// Variant of [msg_super] that skips debug type checking.
///
/// See [msg_unchecked] for why you might need this.
#[macro_export]
macro_rules! msg_super_unchecked {
    [$env:expr; $receiver:tt $name:ident $(: $arg1:tt)?
                             $($namen:ident: $argn:tt)*] => {
        {
            let class = $env.objc.get_known_class(
                _OBJC_CURRENT_CLASS,
                &mut $env.mem
            );
            let sel = $crate::objc::selector!($($arg1;)? $name $(, $namen)*);
            let sel = $env.objc.lookup_selector(sel)
                .expect("Unknown selector");
            let sp = &mut $env.cpu.regs_mut()[$crate::cpu::Cpu::SP];
            let old_sp = *sp;
            *sp -= $crate::mem::guest_size_of::<$crate::objc::objc_super>();
            let super_ptr = $crate::mem::Ptr::from_bits(*sp);
            $env.mem.write(super_ptr, $crate::objc::objc_super {
                receiver: $receiver,
                class,
            });

            let args = (super_ptr, sel, $($arg1,)? $($argn),*);
            let res = $crate::objc::msg_send_super2($env, args);

            $env.cpu.regs_mut()[$crate::cpu::Cpu::SP] = old_sp;

            res
        }
    }
}
pub use crate::msg_super_unchecked; // #[macro_export] is weird...

/// Variant of [msg] for sending a message to a named class. Useful for calling
/// class methods, especially `new`.
///
/// ```ignore
/// msg_class![env; SomeClass alloc]
/// ```
///
/// desugars to:
///
/// ```ignore
/// msg![env; (env.objc.get_known_class("SomeClass", &mut env.mem)) alloc]
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

/// Variant of [msg_class] that skips debug type checking.
///
/// See [msg_unchecked] for why you might need this.
#[macro_export]
macro_rules! msg_class_unchecked {
    [$env:expr; $receiver_class:ident $name:ident $(: $arg1:tt)?
                                      $($namen:ident: $argn:tt)*] => {
        {
            let class = $env.objc.get_known_class(
                stringify!($receiver_class),
                &mut $env.mem
            );
            $crate::objc::msg_unchecked![$env; class $name $(: $arg1)?
                                           $($namen: $argn)*]
        }
    }
}
pub use crate::msg_class_unchecked; // #[macro_export] is weird...

/// Shorthand for `let _: id = msg![env; object retain];`
pub fn retain(env: &mut Environment, object: id) -> id {
    if object == nil {
        // fast path
        return nil;
    }
    msg![env; object retain]
}

/// Shorthand for `() = msg![env; object release];`
pub fn release(env: &mut Environment, object: id) {
    if object == nil {
        // fast path
        return;
    }
    msg![env; object release]
}

/// Shorthand for `let _: id = msg![env; object autorelease];`
pub fn autorelease(env: &mut Environment, object: id) -> id {
    if object == nil {
        // fast path
        return nil;
    }
    msg![env; object autorelease]
}

/// Utility function to generate type ids used in debug type checking.
pub fn generate_type_id<T>(_arg: T) -> u64
where
    T: Any,
{
    // Since we can't pass the TypeId across ABI boundaries, we need to hash it.
    // Instead, we can hash the TypeId and pass that across.
    let tid = TypeId::of::<T>();
    let mut hasher = DefaultHasher::new();
    tid.hash(&mut hasher);
    hasher.finish()
}
