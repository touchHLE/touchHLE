/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSInvocation`.

use crate::abi::{extend_stack_for_args, write_next_arg, GuestArg};
use crate::cpu::Cpu;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::libc::string::strdup;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr};
use crate::msg;
use crate::objc::{
    autorelease, id, nil, objc_classes, objc_msgSend, release, retain, ClassExports, HostObject,
    SEL,
};

struct NSInvocationHostObject {
    /// `NSMethodSignature *`
    sig: id,
    /// Argument type strings resolved from `sig` at creation time
    argument_types: Vec<String>,
    target: id,
    selector: Option<SEL>,
    /// Per-slot owned buffer for each argument.
    /// Option denotes if argument was set with `setArgument:atIndex:`
    arguments: Vec<Option<MutVoidPtr>>,
    arguments_retained: bool,
    /// Objects retained by `retainArguments`
    retained_objects: Vec<id>,
    /// C string copies made by `retainArguments`
    copied_strings: Vec<MutPtr<u8>>,
}
impl HostObject for NSInvocationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSInvocation: NSObject

+ (id)invocationWithMethodSignature:(id)sig { // NSMethodSignature *
    retain(env, sig);
    let num_of_args: NSUInteger = msg![env; sig numberOfArguments];
    let mut argument_types: Vec<String> = Vec::with_capacity(num_of_args as usize);
    for i in 0..num_of_args {
        let type_ptr: ConstPtr<u8> = msg![env; sig getArgumentTypeAtIndex:i];
        argument_types.push(env.mem.cstr_at_utf8(type_ptr).unwrap().to_string());
    }
    let host_object = Box::new(NSInvocationHostObject {
        sig,
        argument_types,
        target: nil,
        selector: None,
        arguments: vec![None; num_of_args as usize],
        arguments_retained: false,
        retained_objects: Vec::new(),
        copied_strings: Vec::new(),
    });
    let res = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, res)
}

- (())setTarget:(id)target {
    let old_target = env.objc.borrow::<NSInvocationHostObject>(this).target;
    let arguments_retained = env.objc.borrow::<NSInvocationHostObject>(this).arguments_retained;
    env.objc.borrow_mut::<NSInvocationHostObject>(this).target = target;
    if arguments_retained {
        retain(env, target);
        release(env, old_target);
    }
}

- (())setSelector:(SEL)selector {
    assert!(env.objc.borrow_mut::<NSInvocationHostObject>(this).selector.is_none()); // TODO
    env.objc.borrow_mut::<NSInvocationHostObject>(this).selector = Some(selector);
}

- (())retainArguments {
    // TODO: handle return val
    // TODO: copy blocks
    assert!(!env.objc.borrow::<NSInvocationHostObject>(this).arguments_retained); // TODO

    let target = env.objc.borrow::<NSInvocationHostObject>(this).target;
    retain(env, target);

    let mut retained_objects: Vec<id> = Vec::new();
    let mut copied_strings: Vec<MutPtr<u8>> = Vec::new();

    // Skip index 0 (self) and 1 (SEL): handled via target/selector fields.
    let num_of_args = env.objc.borrow::<NSInvocationHostObject>(this).argument_types.len();
    for i in 2..num_of_args {
        let host = env.objc.borrow::<NSInvocationHostObject>(this);
        let Some(arg_loc) = host.arguments[i] else { continue };
        match host.argument_types[i].as_str() {
            "@" => {
                let obj: id = env.mem.read(arg_loc.cast().cast_const());
                retain(env, obj);
                retained_objects.push(obj);
            }
            "*" => {
                let str: MutPtr<u8> = env.mem.read(arg_loc.cast().cast_const());
                let str_copy = strdup(env, str.cast_const());
                env.mem.write(arg_loc.cast(), str_copy);
                copied_strings.push(str_copy);
            }
            _ => {}
        }
    }

    let host = env.objc.borrow_mut::<NSInvocationHostObject>(this);
    host.retained_objects = retained_objects;
    host.copied_strings = copied_strings;
    host.arguments_retained = true;
}

- (())setArgument:(MutVoidPtr)arg_loc
          atIndex:(NSInteger)idx {
    let &NSInvocationHostObject {
        ref arguments,
        arguments_retained,
        ..
    } = env.objc.borrow::<NSInvocationHostObject>(this);

    // 0 and 1 are reserved for `self` and `_cmd`
    // TODO: can they be set too?
    assert!(1 < idx && idx < arguments.len() as NSInteger);

    if let Some(prev_arg) = arguments[idx as usize] {
        env.mem.free(prev_arg.cast());
    }

    let argument_types: &Vec<String> = env.objc.borrow::<NSInvocationHostObject>(this).argument_types.as_ref();
    let arg_type = argument_types.get(idx as usize).unwrap();
    let new: MutVoidPtr = match arg_type.as_str() {
        "f" => {
            let arg_loc: MutPtr<f32> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        "@" => {
            assert!(!arguments_retained); // TODO
            let arg_loc: MutPtr<id> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        "*" => {
            assert!(!arguments_retained); // TODO
            let arg_loc: MutPtr<MutPtr<u8>> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        // pointer cases
        _ if arg_type.starts_with('^') => {
            let arg_loc: MutPtr<MutVoidPtr> = arg_loc.cast();
            let arg = env.mem.read(arg_loc);
            env.mem.alloc_and_write(arg).cast()
        }
        _ => unimplemented!("unhandled argument type {arg_type}"),
    };

    env.objc.borrow_mut::<NSInvocationHostObject>(this).arguments[idx as usize] = Some(new);
}

- (())invokeWithTarget:(id)target {
    () = msg![env; this setTarget:target];
    () = msg![env; this invoke];
}

- (())invoke {
    // Safeguard: all arguments must be set (except first two)
    let arguments: &Vec<Option<MutVoidPtr>> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
    let set_count = arguments.iter().flatten().count();
    let all_count = arguments.len();
    assert_eq!(set_count + 2, all_count);

    let sig = env.objc.borrow::<NSInvocationHostObject>(this).sig;
    let ret_type: ConstPtr<u8> = msg![env; sig methodReturnType];
    assert!(env.mem.read(ret_type) == b'v'); // TODO

    // `call_from_host` re-use
    // TODO: retval_ptr
    // TODO: cross check against frame length from NSMethodSignature
    let mut reg_count = 0;
    let argument_types: &Vec<String> = env.objc.borrow::<NSInvocationHostObject>(this).argument_types.as_ref();
    for arg_type in argument_types.iter() {
        // TODO: refactor and simplify
        reg_count += match arg_type.as_str() {
            "@" => <id as GuestArg>::REG_COUNT,
            ":" => <SEL as GuestArg>::REG_COUNT,
            "f" => <f32 as GuestArg>::REG_COUNT,
            "c" => <u8 as GuestArg>::REG_COUNT,
            "*" => <MutPtr<u8> as GuestArg>::REG_COUNT,
            // pointer cases
            _ if arg_type.starts_with('^') => <MutVoidPtr as GuestArg>::REG_COUNT,
            _ => unimplemented!("reg_count for {arg_type}")
        }
    }
    let regs = env.cpu.regs_mut();
    let old_sp = extend_stack_for_args(
        reg_count,
        regs,
    );

    let arguments: &Vec<Option<MutVoidPtr>> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
    let mut reg_offset = 0;
    for i in 0..arguments.len() {
        // TODO: do not handle target and sel as special cases
        if i == 0 {
            assert!(argument_types[i] == "@");
            // target
            let target = env.objc.borrow::<NSInvocationHostObject>(this).target;
            let regs = env.cpu.regs_mut();
            write_next_arg::<id>(&mut reg_offset, regs, &mut env.mem, target);
            continue;
        }
        if i == 1 {
            assert!(argument_types[i] == ":");
            // selector
            let selector = env.objc.borrow::<NSInvocationHostObject>(this).selector.unwrap();
            let regs = env.cpu.regs_mut();
            write_next_arg::<SEL>(&mut reg_offset, regs, &mut env.mem, selector);
            continue;
        }
        let arg_slot = arguments[i].unwrap();
        let arg_type = argument_types[i].as_str();
        // TODO: refactor and simplify
        match arg_type {
            "@" => {
                let arg: ConstPtr<id> = arg_slot.cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<id>(&mut reg_offset, regs, &mut env.mem, arg_val);
            },
            "f" => {
                let arg: ConstPtr<f32> = arg_slot.cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<f32>(&mut reg_offset, regs, &mut env.mem, arg_val);
            },
            "c" => {
                let arg: ConstPtr<u8> = arg_slot.cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<u8>(&mut reg_offset, regs, &mut env.mem, arg_val);
            }
            "*" => {
                let arg: ConstPtr<MutPtr<u8>> = arg_slot.cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<MutPtr<u8>>(&mut reg_offset, regs, &mut env.mem, arg_val);
            }
            // pointer cases
            _ if arg_type.starts_with('^') => {
                let arg: ConstPtr<MutVoidPtr> = arg_slot.cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<MutVoidPtr>(&mut reg_offset, regs, &mut env.mem, arg_val);
            }
            _ => unimplemented!("write_next_arg for {arg_type}")
        }
    }

    // actual invocation
    let &NSInvocationHostObject { target, selector, .. } = env.objc.borrow::<NSInvocationHostObject>(this);
    objc_msgSend(env, target, selector.unwrap());

    let regs = env.cpu.regs_mut(); // re-borrow
    regs[Cpu::SP] = old_sp;
    // TODO: non-void return
}

- (())dealloc {
    let &NSInvocationHostObject { sig, target, arguments_retained, .. } = env.objc.borrow::<NSInvocationHostObject>(this);
    release(env, sig);
    if arguments_retained {
        release(env, target);
        let retained_objects = std::mem::take(
            &mut env.objc.borrow_mut::<NSInvocationHostObject>(this).retained_objects
        );
        for obj in retained_objects {
            release(env, obj);
        }
        let copied_strings = std::mem::take(
            &mut env.objc.borrow_mut::<NSInvocationHostObject>(this).copied_strings
        );
        for s in copied_strings {
            env.mem.free(s.cast());
        }
    } else {
        assert!(env.objc.borrow::<NSInvocationHostObject>(this).retained_objects.is_empty());
        assert!(env.objc.borrow::<NSInvocationHostObject>(this).copied_strings.is_empty());
    }
    for ptr in env.objc.borrow::<NSInvocationHostObject>(this).arguments.iter().flatten() {
        env.mem.free(ptr.cast());
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
