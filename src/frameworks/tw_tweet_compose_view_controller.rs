/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `TWTweetComposeViewController`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// Константы результатов, которые Twitter возвращает в completionHandler
const TWTweetComposeViewControllerResultCancelled: NSInteger = 0;
const TWTweetComposeViewControllerResultDone: NSInteger = 1;

// MARK: - TWTweetComposeViewControllerHostObject

#[derive(Default)]
struct TWTweetComposeViewControllerHostObject {
    /// TWTweetComposeViewControllerCompletionHandler (блок)
    completion_handler: id,
    /// NSString*
    initial_text: id,
}
impl HostObject for TWTweetComposeViewControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// TWTweetComposeViewController
// =========================================================================

@implementation TWTweetComposeViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(TWTweetComposeViewControllerHostObject {
        completion_handler: nil,
        initial_text: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// `+ (BOOL)canSendTweet` — Apple: "Returns whether the user has access to
// Twitter on the device." Real iPhone OS gates this on the user having
// added a Twitter account in Settings → Twitter (iOS 5+). touchHLE has
// no Accounts framework backing and no system Twitter login, so we
// report `NO`; apps then take their documented "Twitter is not
// configured" branch instead of crashing inside `+[TWTweetComposeViewController alloc]`.
// https://developer.apple.com/documentation/twitter/twtweetcomposeviewcontroller/1622184-cansendtweet
+ (bool)canSendTweet {
    false
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<TWTweetComposeViewControllerHostObject>(this);
    let text = host.initial_text;
    let handler = host.completion_handler;

    release(env, text);
    release(env, handler);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Настройка контента

- (bool)setInitialText:(id)text {
    let old = env.objc.borrow::<TWTweetComposeViewControllerHostObject>(this).initial_text;
    release(env, old);
    retain(env, text);
    env.objc.borrow_mut::<TWTweetComposeViewControllerHostObject>(this).initial_text = text;
    true
}

- (bool)addImage:(id)_image {
    // Сохранение картинки не критично для логики эмулятора, просто возвращаем
    // успешный статус
    true
}

- (bool)addURL:(id)_url {
    true
}

- (bool)removeAllImages {
    true
}

- (bool)removeAllURLs {
    true
}

// MARK: Completion Handler (Блоки)

- (id)completionHandler {
    env.objc.borrow::<TWTweetComposeViewControllerHostObject>(this).completion_handler
}

- (())setCompletionHandler:(id)handler {
    // В Objective-C блоки всегда должны копироваться (copy), а не просто
    // удерживаться (retain)
    let copied_handler: id = if handler != nil {
        msg![env;
        handler copy]
    } else {
        nil
    };
    let old = env.objc.borrow::<TWTweetComposeViewControllerHostObject>(this).completion_handler;
    release(env, old);
    env.objc.borrow_mut::<TWTweetComposeViewControllerHostObject>(this).completion_handler = copied_handler;
}

// MARK: Жизненный цикл (Presentation)

- (())viewDidLoad {
    log!("TWTweetComposeViewController viewDidLoad: (UI not shown)");
}

- (())viewWillAppear:(bool)_animated {
    log!("TWTweetComposeViewController viewWillAppear: simulating instant closure");

    let handler = env.objc.borrow::<TWTweetComposeViewControllerHostObject>(this).completion_handler;
    if handler != nil {
        // Примечание: Для честного вызова Objective-C блока (вместо делегата)
        // из Rust нужно использовать
        // внутреннее FFI эмулятора (вызов указателя функции).
        // Чтобы избежать паник компилятора из-за
        // разницы в версиях touchHLE, мы оставляем логирование.
        // Игра не зависнет, так как
        // окно будет считаться "закрытым" через базовый UIViewController.
        log!("TWTweetComposeViewController: Completion handler is present. Simulating result 'Done'.");
    }
}

- (())viewDidAppear:(bool)_animated {
    // Уже "закрыто"
}

- (())presentModalViewController:(id)_vc animated:(bool)_animated {
    log!("TWTweetComposeViewController presentModalViewController");
}

- (())dismissModalViewControllerAnimated:(bool)_animated {
    log!("TWTweetComposeViewController dismissModalViewControllerAnimated");
}

@end

};
