/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAAnimation` and its subclasses
use std::collections::HashMap;

use crate::dyld::{ConstantExports, HostConstant};
use crate::environment::Environment;
use crate::frameworks::core_animation::ca_media_timing_function::kCAMediaTimingFunctionDefault;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::objc::{id, nil, objc_classes, release, retain, ClassExports};
use crate::{msg, msg_class};

#[derive(Default)]
pub struct State {
    /// List of views for internal purposes. Non-retaining!
    transactions: Vec<Transaction>,
}
impl State {
    pub fn get(env: &mut Environment) -> &State {
        &env.framework_state.core_animation.ca_transaction
    }

    pub fn get_mut(env: &mut Environment) -> &mut State {
        &mut env.framework_state.core_animation.ca_transaction
    }

    pub fn add_animation(env: &mut Environment, layer: id, animation: id) {
        if let Some(transaction) = State::get_mut(env).transactions.last_mut() {
            transaction.animations.push((layer, animation));
        } else {
            () = msg_class![env; CATransaction begin];
            State::get_mut(env).transactions.last_mut().unwrap().animations.push((layer, animation));
            () = msg_class![env; CATransaction commit];
        };
    }
}

pub struct Transaction {
    disable_actions: bool,
    animation_duration: CFTimeInterval,
    animation_timing_function: id, // CAMediaTimingFunction*
    data: HashMap<String, id>,
    pub animations: Vec<(id, id)>, // CALayer*, CAAnimation*
}
impl Default for Transaction {
    fn default() -> Self {
        Self {
            disable_actions: false,
            animation_duration: 0.25,
            animation_timing_function: nil,
            data: HashMap::default(),
            animations: Vec::default()
        }
    }
}

pub const kCATransactionAnimationDuration: &str = "kCATransactionAnimationDuration";
pub const kCATransactionDisableActions: &str = "kCATransactionDisableActions";
pub const kCATransactionAnimationTimingFunction: &str = "kCATransactionAnimationTimingFunction";
pub const kCATransactionCompletionBlock: &str = "kCATransactionCompletionBlock";

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
    if key_string == kCATransactionAnimationDuration {
        let value: CFTimeInterval = msg![env; value doubleValue];
        State::get_mut(env).transactions.last_mut().unwrap().animation_duration = value;
    } else if key_string == kCATransactionDisableActions {
        let value: bool = msg![env; value boolValue];
        State::get_mut(env).transactions.last_mut().unwrap().disable_actions = value;
    } else if key_string == kCATransactionAnimationTimingFunction {
        let transaction = State::get_mut(env).transactions.last_mut().unwrap();
        let old_value = std::mem::replace(&mut transaction.animation_timing_function, value);
        retain(env, value);
        release(env, old_value);
    } else if key_string == kCATransactionCompletionBlock {
        unimplemented!();
    } else {
        let transaction = State::get_mut(env).transactions.last_mut().unwrap();
        let old_value = transaction.data.insert(key_string.to_string(), value).unwrap_or(nil);
        retain(env, value);
        release(env, old_value);
    }
    log_dbg!("[CATransaction setValue:{:?} forKey:{:?} ({})]", value, key, key_string);
}
+ (id)valueForKey:(id)key { // NSString*
    let key_string = to_rust_string(env, key);
    let value = if key_string == kCATransactionAnimationDuration {
        let animation_duration = State::get(env).transactions.last().unwrap().animation_duration;
        msg_class![env; NSNumber numberWithDouble:animation_duration]
    } else if key_string == kCATransactionDisableActions {
        let disable_actions = State::get(env).transactions.last().unwrap().disable_actions;
        msg_class![env; NSNumber numberWithBool:disable_actions]
    } else if key_string == kCATransactionAnimationTimingFunction {
        State::get(env).transactions.last().unwrap().animation_timing_function
    } else if key_string == kCATransactionCompletionBlock {
        unimplemented!()
    } else {
        State::get(env).transactions.last().unwrap().data.get(&*key_string).cloned().unwrap_or(nil)
    };
    log_dbg!("[CATransaction valueForKey:{:?} ({})] => {:?}", key, key_string, value);
    value
}

+ (())begin {
    log_dbg!("[CATransaction begin]");
    let animation_timing_function_name = get_static_str(env, kCAMediaTimingFunctionDefault);
    let animation_timing_function = msg_class![env; CAMediaTimingFunction functionWithName:animation_timing_function_name];
    State::get_mut(env).transactions.push(Transaction {
        animation_duration: 0.25,
        animation_timing_function,
        ..Default::default()
    });
}

+ (())commit {
    log_dbg!("[CATransaction commit]");
    let transaction = State::get_mut(env).transactions.pop().unwrap();
    for (layer, animation) in transaction.animations {
        if transaction.disable_actions {
            release(env, animation);
        } else {
            () = msg![env; animation setDuration: (transaction.animation_duration)];
            () = msg![env; animation setTimingFunction: (transaction.animation_timing_function)];
            () = msg![env; layer addAnimation: animation forKey: nil];
        }
    }
}

+ (bool)disableActions {
    let key = get_static_str(env, kCATransactionDisableActions);
    msg![env; this valueForKey: key]
}
+ (())setDisableActions:(bool)flag {
    let value: id = msg_class![env; NSNumber numberWithBool: flag];
    let key = get_static_str(env, kCATransactionDisableActions);
    msg![env; this setValue: value forKey: key]
}

+ (CFTimeInterval)animationDuration {
    let key = get_static_str(env, kCATransactionAnimationDuration);
    msg![env; this valueForKey: key]
}
+ (())setAnimationDuration:(CFTimeInterval)duration {
    let value: id = msg_class![env; NSNumber numberWithDouble: duration];
    let key = get_static_str(env, kCATransactionAnimationDuration);
    msg![env; this setValue: value forKey: key]
}

+ (id) animationTimingFunction {
    let key = get_static_str(env, kCATransactionAnimationTimingFunction);
    msg![env; this valueForKey: key]
}
+ (())setAnimationTimingFunction:(id)animation_timing_function { // CAMediaTimingFunction *
    let key = get_static_str(env, kCATransactionAnimationTimingFunction);
    msg![env; this setValue: animation_timing_function forKey: key]
}
@end

};
