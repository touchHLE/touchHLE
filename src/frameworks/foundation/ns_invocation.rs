/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSInvocation`.

use crate::abi::{extend_stack_for_args, write_next_arg, GuestArg, GuestRet};
use crate::cpu::Cpu;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr};
use crate::msg;
use crate::objc::{
    autorelease, id, nil, objc_classes, objc_msgSend, release, retain, ClassExports, HostObject,
    SEL,
};

struct NSInvocationHostObject {
    /// `NSMethodSignature *`
    sig: id,
    target: id,
    selector: Option<SEL>,
    arguments: Vec<MutVoidPtr>,
    arguments_retained: bool,
}
impl HostObject for NSInvocationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSInvocation: NSObject

+ (id)invocationWithMethodSignature:(id)sig { // NSMethodSignature *
    retain(env, sig);
    let num_of_args: NSUInteger = msg![env; sig numberOfArguments];
    let host_object = Box::new(NSInvocationHostObject {
        sig,
        target: nil,
        selector: None,
        arguments: vec![MutPtr::null(); num_of_args as usize],
        arguments_retained: false,
    });
    let res = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, res)
}

- (())setTarget:(id)target {
    env.objc.borrow_mut::<NSInvocationHostObject>(this).target = target;
}

- (())setSelector:(SEL)selector {
    assert!(env.objc.borrow_mut::<NSInvocationHostObject>(this).selector.is_none()); // TODO
    env.objc.borrow_mut::<NSInvocationHostObject>(this).selector = Some(selector);
}

- (())retainArguments {
    let target = env.objc.borrow_mut::<NSInvocationHostObject>(this).target;
    retain(env, target);
    // TODO retain all args and copy C Strings
    env.objc.borrow_mut::<NSInvocationHostObject>(this).arguments_retained = true;
}

- (())setArgument:(MutVoidPtr)arg_loc
          atIndex:(NSInteger)idx {
    env.objc.borrow_mut::<NSInvocationHostObject>(this).arguments[idx as usize] = arg_loc;
}

- (())invokeWithTarget:(id)target {
    () = msg![env; this setTarget:target];
    () = msg![env; this invoke];
}

- (())invoke {
    let sig = env.objc.borrow::<NSInvocationHostObject>(this).sig;
    let ret_type: ConstPtr<u8> = msg![env; sig methodReturnType];
    assert!(env.mem.read(ret_type) == b'v'); // TODO

    // TODO: move to init?
    let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
    let mut argument_types = Vec::new();
    for i in 0..arguments.len() as u32 {
        let sig = env.objc.borrow::<NSInvocationHostObject>(this).sig;
        let arg_type_str: ConstPtr<u8> = msg![env; sig getArgumentTypeAtIndex:i];
        let arg_type = env.mem.cstr_at_utf8(arg_type_str).unwrap();
        argument_types.push(arg_type.to_string());
    }

    // `call_from_host` re-use
    // TODO: retval_ptr
    let mut reg_count = 0;
    let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
    for i in 0..arguments.len() {
        let arg_type = argument_types[i].as_str();
        reg_count += match arg_type {
            "@" => <id as GuestArg>::REG_COUNT,
            ":" => <SEL as GuestArg>::REG_COUNT,
            "f" => <f32 as GuestArg>::REG_COUNT,
            // TODO: generalize pointer handling
            "^v" => <MutVoidPtr as GuestArg>::REG_COUNT,
            "c" => <u8 as GuestArg>::REG_COUNT,
            _ => unimplemented!("reg_count for {arg_type}")
        }
    }
    let regs = env.cpu.regs_mut();
    let old_sp = extend_stack_for_args(
        reg_count,
        regs,
    );

    let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
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
        let arg_type = argument_types[i].as_str();
        match arg_type {
            "@" => {
                let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
                let arg: ConstPtr<id> = arguments[i].cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<id>(&mut reg_offset, regs, &mut env.mem, arg_val);
            },
            "f" => {
                let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
                let arg: ConstPtr<f32> = arguments[i].cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<f32>(&mut reg_offset, regs, &mut env.mem, arg_val);
            },
            "^v" => {
                let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
                let arg: ConstPtr<MutVoidPtr> = arguments[i].cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<MutVoidPtr>(&mut reg_offset, regs, &mut env.mem, arg_val);
            }
            "c" => {
                let arguments: &Vec<MutVoidPtr> = env.objc.borrow::<NSInvocationHostObject>(this).arguments.as_ref();
                let arg: ConstPtr<u8> = arguments[i].cast().cast_const();
                let arg_val = env.mem.read(arg);
                let regs = env.cpu.regs_mut();
                write_next_arg::<u8>(&mut reg_offset, regs, &mut env.mem, arg_val);
            }
            _ => unimplemented!("write_next_arg for {arg_type}")
        }
    }

    // actual invocation
    let &NSInvocationHostObject { target, selector, .. } = env.objc.borrow::<NSInvocationHostObject>(this);
    objc_msgSend(env, target, selector.unwrap());

    let regs = env.cpu.regs_mut(); // reborrow
    regs[Cpu::SP] = old_sp;
    // TODO: non-void return
}

- (())dealloc {
    let &NSInvocationHostObject { sig, target, arguments_retained, .. } = env.objc.borrow::<NSInvocationHostObject>(this);
    release(env, sig);
    if arguments_retained {
        release(env, target);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
