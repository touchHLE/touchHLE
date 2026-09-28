/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frameworks::foundation::{ns_array, NSInteger};
use crate::objc::{
    id, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};

struct SKPaymentQueueHostObject {
    observer: id,
}
impl HostObject for SKPaymentQueueHostObject {}

struct SKPaymentTransactionHostObject {
    payment: id,
}
impl HostObject for SKPaymentTransactionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation SKPaymentQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(SKPaymentQueueHostObject { observer: nil }),
        &mut env.mem
    )
}

+ (id)defaultQueue {
    if let Some(queue) = crate::frameworks::store_kit::State::get(&mut env.framework_state)
        .payment_queue
    {
        return queue;
    }

    let queue = env.objc.alloc_static_object(
        this,
        Box::new(SKPaymentQueueHostObject { observer: nil }),
        &mut env.mem,
    );
    crate::frameworks::store_kit::State::get(&mut env.framework_state).payment_queue = Some(queue);
    queue
}

+ (bool)canMakePayments {
    false
}

- (())addTransactionObserver:(id)new_observer {
    log!("StoreKit transaction observer set to {:?}.", new_observer);
    env.objc
        .borrow_mut::<SKPaymentQueueHostObject>(this)
        .observer = new_observer;
}

- (())removeTransactionObserver:(id)_observer {}

- (())addPayment:(id)payment {
    // Intentionally report completion without granting the product. This lets
    // apps dismiss their App Store UI while keeping IAP unavailable.
    log!("Ignoring StoreKit payment and reporting completion without granting it.");
    let observer = env
        .objc
        .borrow::<SKPaymentQueueHostObject>(this)
        .observer;
    if observer == nil {
        return;
    }

    let transaction: id = msg_class![env; SKPaymentTransaction alloc];
    let transaction: id = msg![env; transaction initWithPayment:payment];
    let transactions = ns_array::from_vec(env, vec![transaction]);
    let selector: SEL = env.objc.register_host_selector(
        "paymentQueue:updatedTransactions:".to_string(),
        &mut env.mem,
    );
    let responds: bool = msg![env; observer respondsToSelector:selector];
    log!(
        "StoreKit transaction observer responds to updates: {}.",
        responds
    );
    if responds {
        () = msg![env; observer paymentQueue:this updatedTransactions:transactions];
    }
}

- (())finishTransaction:(id)_transaction {}

@end

@implementation SKPayment: NSObject

+ (id)paymentWithProductIdentifier:(id)_identifier {
    log!("Ignoring StoreKit payment request.");
    nil
}

@end

@implementation SKPaymentTransaction: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(SKPaymentTransactionHostObject { payment: nil }),
        &mut env.mem
    )
}

- (id)initWithPayment:(id)new_payment {
    let this = msg![env; this init];
    retain(env, new_payment);
    env.objc
        .borrow_mut::<SKPaymentTransactionHostObject>(this)
        .payment = new_payment;
    this
}

- (id)payment {
    env.objc
        .borrow::<SKPaymentTransactionHostObject>(this)
        .payment
}

- (NSInteger)transactionState {
    1 // SKPaymentTransactionStatePurchased
}

- (id)error {
    nil
}

- (id)transactionIdentifier {
    nil
}

- (())dealloc {
    let payment = env
        .objc
        .borrow::<SKPaymentTransactionHostObject>(this)
        .payment;
    release(env, payment);
    msg_super![env; this dealloc]
}

@end

};
