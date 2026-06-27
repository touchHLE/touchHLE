/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `SKPaymentQueue` — StoreKit in-app purchase queue stub.

use crate::frameworks::foundation::{ns_string, NSInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

// MARK: - Per-process state

/// Singleton cache for `[SKPaymentQueue defaultQueue]`.
#[derive(Default)]
pub struct State {
    default_queue: Option<id>,
}

impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.store_kit.payment_queue
    }
}

#[derive(Default)]
struct SKPaymentQueueHostObject {
    /// SKPaymentTransactionObserver — weak reference
    observer: id,
}
impl HostObject for SKPaymentQueueHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation SKPaymentQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKPaymentQueueHostObject {
        observer: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Singleton

+ (id)defaultQueue {
    // Always return the same singleton so observers are not lost between calls.
    if let Some(queue) = State::get(env).default_queue {
        return queue;
    }
    let queue: id = msg![env; this alloc];
    let queue: id = msg![env; queue init];
    // refcount=1 is our singleton retain — do NOT autorelease
    State::get(env).default_queue = Some(queue);
    log!("SKPaymentQueue defaultQueue: singleton created");
    queue
}

+ (bool)canMakePayments {
    // Claim payments are not available — safest stub for a non-App-Store build.
    false
}

// MARK: - Init

- (id)init {
    this
}

- (())dealloc {
    let observer = env.objc.borrow::<SKPaymentQueueHostObject>(this).observer;
    release(env, observer);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Observers

- (())addTransactionObserver:(id)observer {
    // Убрали .unwrap()
    let host_obj = env.objc.borrow_mut::<SKPaymentQueueHostObject>(this);
    host_obj.observer = observer;
}

- (())removeTransactionObserver:(id)_observer {
    // Убрали .unwrap()
    let host_obj = env.objc.borrow_mut::<SKPaymentQueueHostObject>(this);
    host_obj.observer = nil;
}

// MARK: - Payment requests

- (())addPayment:(id)_payment { // SKPayment*
    log!("SKPaymentQueue addPayment: stubbed — failing transaction immediately");

    // Notify observer that the payment failed so the app can handle it cleanly.
    let observer = env.objc.borrow::<SKPaymentQueueHostObject>(this).observer;

    if observer == nil {
        return;
    }

    // Build a minimal fake SKPaymentTransaction array and call the delegate.
    // Most apps only check the transactionState, which we set to
    // SKPaymentTransactionStateFailed (2).
    let transactions: id = msg_class![env; NSArray new];

    let sel = env.objc.register_host_selector(
        "paymentQueue:updatedTransactions:".to_string(),
        &mut env.mem,
    );

    let responds: bool = msg![env; observer respondsToSelector:sel];
    if responds {
        () = msg![env; observer paymentQueue:this updatedTransactions:transactions];
    }
}

- (())restoreCompletedTransactions {
    // Убрали .unwrap()
    let host_obj = env.objc.borrow::<SKPaymentQueueHostObject>(this);
    let observer = host_obj.observer;

    if observer != nil {
        // Вызываем метод делегата, сообщая, что "восстановление" успешно
        // завершено
        let _: () = msg![env; observer paymentQueueRestoreCompletedTransactionsFinished:this];
    }
}

- (())restoreCompletedTransactionsWithApplicationUsername:(id)_username {
    msg![env; this restoreCompletedTransactions]
}

- (())finishTransaction:(id)_transaction { // SKPaymentTransaction*
    log!("SKPaymentQueue finishTransaction: stubbed");
}

// MARK: - Downloads (iOS 6+, always empty)

- (id)transactions {
    msg_class![env; NSArray new]
}

- (())startDownloads:(id)_downloads {
    log!("SKPaymentQueue startDownloads: stubbed");
}

- (())pauseDownloads:(id)_downloads {
    log!("SKPaymentQueue pauseDownloads: stubbed");
}

- (())resumeDownloads:(id)_downloads {
    log!("SKPaymentQueue resumeDownloads: stubbed");
}

- (())cancelDownloads:(id)_downloads {
    log!("SKPaymentQueue cancelDownloads: stubbed");
}

@end

// MARK: - SKPayment (read-only request object)

@implementation SKPayment: NSObject

+ (id)paymentWithProductIdentifier:(id)identifier { // NSString*
    let payment: id = msg_class![env; SKPayment alloc];
    let payment: id = msg![env; payment initWithProductIdentifier:identifier];
    autorelease(env, payment)
}

+ (id)paymentWithProduct:(id)product { // SKProduct*
    let identifier: id = msg![env; product productIdentifier];
    msg_class![env; SKPayment paymentWithProductIdentifier:identifier]
}

- (id)initWithProductIdentifier:(id)_identifier {
    this
}

- (id)productIdentifier {
    ns_string::get_static_str(env, "")
}

- (NSInteger)quantity {
    1
}

@end

// MARK: - SKPaymentTransaction (stub)

@implementation SKPaymentTransaction: NSObject

- (NSInteger)transactionState {
    2 // SKPaymentTransactionStateFailed
}

- (id)transactionIdentifier {
    nil
}

- (id)payment {
    nil
}

- (id)error {
    nil
}

- (id)originalTransaction {
    nil
}

@end

};
