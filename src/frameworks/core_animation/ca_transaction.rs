/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAAnimation` and its subclasses
use std::collections::HashMap;

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_animation::ca_media_timing_function::kCAMediaTimingFunctionDefault;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::objc::{id, nil, objc_classes, release, retain, ClassExports};
use crate::{msg, msg_class};
use crate::{Environment, ThreadId};

#[derive(Default)]
pub struct State {
    // TODO: Clean up state from threads that finish
    transactions: HashMap<ThreadId, ThreadState>,
}
impl State {
    pub fn get(env: &mut Environment) -> &State {
        &env.framework_state.core_animation.ca_transaction
    }

    pub fn get_mut(env: &mut Environment) -> &mut State {
        &mut env.framework_state.core_animation.ca_transaction
    }

    pub fn get_current_transaction(env: &mut Environment) -> Option<&Transaction> {
        let current_thread = env.current_thread;
        State::get(env)
            .transactions
            .get(&current_thread)
            .and_then(|t| t.get_current_transaction())
    }

    fn get_current_transaction_mut(env: &mut Environment) -> Option<&mut Transaction> {
        let current_thread = env.current_thread;
        State::get_mut(env)
            .transactions
            .get_mut(&current_thread)
            .and_then(|t| t.get_current_transaction_mut())
    }

    // Unused until support for implicit animations is implemented.
    #[allow(unused)]
    pub fn add_animation(env: &mut Environment, layer: id, animation: id) {
        let layer_class = msg![env; layer class];
        let ca_layer = env.objc.get_known_class("CALayer", &mut env.mem);
        assert!(env.objc.class_is_subclass_of(layer_class, ca_layer));
        let anim_class = msg![env; animation class];
        let ca_animation = env.objc.get_known_class("CAAnimation", &mut env.mem);
        retain(env, layer);
        retain(env, animation);
        assert!(env.objc.class_is_subclass_of(anim_class, ca_animation));
        if let Some(transaction) = State::get_current_transaction_mut(env) {
            transaction.add_animation(layer, animation);
        } else {
            let mut transaction = Transaction::new(env);
            transaction.add_animation(layer, animation);
            let thread_state = State::get_current_thread_state_mut(env);
            assert!(thread_state.implicit_transaction.is_none());
            thread_state.implicit_transaction = Some(transaction);
        };
    }

    pub fn commit_implicit_transaction(env: &mut Environment) {
        let state = State::get_current_thread_state_mut(env);
        assert!(state.explicit_transactions.is_empty()); // TODO: Verify what should happen
        if let Some(transaction) = std::mem::take(&mut state.implicit_transaction) {
            transaction.commit(env);
        }
    }

    fn get_current_thread_state_mut(env: &mut Environment) -> &mut ThreadState {
        let current_thread = env.current_thread;
        State::get_mut(env)
            .transactions
            .entry(current_thread)
            .or_default()
    }

    fn push_explicit_transaction(env: &mut Environment) {
        let transaction = Transaction::new(env);
        State::get_current_thread_state_mut(env)
            .explicit_transactions
            .push(transaction);
    }

    fn pop_explicit_transaction(env: &mut Environment) -> Transaction {
        let current_thread = env.current_thread;
        State::get_mut(env)
            .transactions
            .get_mut(&current_thread)
            .unwrap()
            .explicit_transactions
            .pop()
            .unwrap()
    }
}

#[derive(Default)]
struct ThreadState {
    implicit_transaction: Option<Transaction>,
    explicit_transactions: Vec<Transaction>,
}
impl ThreadState {
    fn get_current_transaction(&self) -> Option<&Transaction> {
        self.explicit_transactions
            .last()
            .or(self.implicit_transaction.as_ref())
    }

    fn get_current_transaction_mut(&mut self) -> Option<&mut Transaction> {
        self.explicit_transactions
            .last_mut()
            .or(self.implicit_transaction.as_mut())
    }
}

pub struct Transaction {
    disable_actions: bool,
    animation_duration: CFTimeInterval,
    animation_timing_function: id, // CAMediaTimingFunction*
    data: HashMap<String, id>,
    animations: Vec<(id, id)>, // CALayer*, CAAnimation*
}
impl Transaction {
    fn new(env: &mut Environment) -> Self {
        let animation_timing_function_name = get_static_str(env, kCAMediaTimingFunctionDefault);
        let animation_timing_function =
            msg_class![env; CAMediaTimingFunction functionWithName:animation_timing_function_name];
        Self {
            disable_actions: false,
            animation_duration: 0.25,
            animation_timing_function,
            data: HashMap::default(),
            animations: Vec::default(),
        }
    }

    // Unused until support for UIView animations is implemented.
    #[allow(unused)]
    pub fn get_animations(&self) -> Vec<(id, id)> {
        self.animations.clone()
    }

    fn add_animation(&mut self, layer: id, animation: id) {
        self.animations.push((layer, animation));
    }

    fn commit(self, env: &mut Environment) {
        for (layer, animation) in self.animations {
            if !self.disable_actions {
                () = msg![env; animation setDuration: (self.animation_duration)];
                () = msg![env; animation setTimingFunction: (self.animation_timing_function)];
                () = msg![env; layer addAnimation: animation forKey: nil];
            }
            release(env, layer);
            release(env, animation);
        }

        for (_key, value) in self.data {
            release(env, value);
        }
    }
}

pub const kCATransactionAnimationDuration: &str = "animationDuration";
pub const kCATransactionDisableActions: &str = "disableActions";
pub const kCATransactionAnimationTimingFunction: &str = "animationTimingFunction";
pub const kCATransactionCompletionBlock: &str = "completionBlock";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCATransactionAnimationDuration",
        HostConstant::NSString(kCATransactionAnimationDuration),
    ),
    (
        "_kCATransactionDisableActions",
        HostConstant::NSString(kCATransactionDisableActions),
    ),
    (
        "_kCATransactionAnimationTimingFunction",
        HostConstant::NSString(kCATransactionAnimationTimingFunction),
    ),
    (
        "_kCATransactionCompletionBlock",
        HostConstant::NSString(kCATransactionCompletionBlock),
    ),
];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CATransaction: NSObject

+ (())setValue:(id)value forKey:(id)key {
    let key_string = to_rust_string(env, key);
    log_dbg!("[CATransaction setValue:{:?} forKey:{:?} ({})]", value, key, key_string);
    match &*key_string  {
        kCATransactionAnimationDuration => {
            let value: CFTimeInterval = msg![env; value doubleValue];
            State::get_current_transaction_mut(env).unwrap().animation_duration = value;
        },
        kCATransactionDisableActions => {
            let value: bool = msg![env; value boolValue];
            State::get_current_transaction_mut(env).unwrap().disable_actions = value;
        },
        kCATransactionAnimationTimingFunction => {
            let transaction = State::get_current_transaction_mut(env).unwrap();
            let old_value = std::mem::replace(&mut transaction.animation_timing_function, value);
            retain(env, value);
            release(env, old_value);
        },
        kCATransactionCompletionBlock => {
            unimplemented!();
        },
        _ => {
            let transaction = State::get_current_transaction_mut(env).unwrap();
            let old_value = transaction.data.insert(key_string.to_string(), value).unwrap_or(nil);
            retain(env, value);
            release(env, old_value);
        }
    };
}
+ (id)valueForKey:(id)key { // NSString*
    let key_string = to_rust_string(env, key);
    let value = match &*key_string {
        kCATransactionAnimationDuration => {
            let animation_duration = State::get_current_transaction(env).unwrap().animation_duration;
            msg_class![env; NSNumber numberWithDouble:animation_duration]
        },
        kCATransactionDisableActions => {
            let disable_actions = State::get_current_transaction(env).unwrap().disable_actions;
            msg_class![env; NSNumber numberWithBool:disable_actions]
        },
        kCATransactionAnimationTimingFunction => {
            State::get_current_transaction(env).unwrap().animation_timing_function
        },
        kCATransactionCompletionBlock => {
            unimplemented!()
        },
        _ => {
            State::get_current_transaction(env).unwrap().data.get(&*key_string).cloned().unwrap_or(nil)
        }
    };
    log_dbg!("[CATransaction valueForKey:{:?} ({})] => {:?}", key, key_string, value);
    value
}

+ (())begin {
    log_dbg!("[CATransaction begin]");
    State::push_explicit_transaction(env);
}

+ (())commit {
    log_dbg!("[CATransaction commit]");
    State::pop_explicit_transaction(env).commit(env);
}

+ (bool)disableActions {
    let key = get_static_str(env, kCATransactionDisableActions);
    let value = msg![env; this valueForKey: key];
    msg![env; value boolValue]
}
+ (())setDisableActions:(bool)flag {
    let value: id = msg_class![env; NSNumber numberWithBool: flag];
    let key = get_static_str(env, kCATransactionDisableActions);
    msg![env; this setValue: value forKey: key]
}

+ (CFTimeInterval)animationDuration {
    let key = get_static_str(env, kCATransactionAnimationDuration);
    let value = msg![env; this valueForKey: key];
    msg![env; value doubleValue]
}
+ (())setAnimationDuration:(CFTimeInterval)duration {
    let value: id = msg_class![env; NSNumber numberWithDouble: duration];
    let key = get_static_str(env, kCATransactionAnimationDuration);
    msg![env; this setValue: value forKey: key]
}

+ (id)animationTimingFunction {
    let key = get_static_str(env, kCATransactionAnimationTimingFunction);
    msg![env; this valueForKey: key]
}
+ (())setAnimationTimingFunction:(id)animation_timing_function { // CAMediaTimingFunction *
    let key = get_static_str(env, kCATransactionAnimationTimingFunction);
    msg![env; this setValue: animation_timing_function forKey: key]
}
@end

};
