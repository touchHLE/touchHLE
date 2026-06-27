/*
 * FBSession stub for touchHLE
 */

use crate::objc::{id, msg, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
struct FBSessionHostObject {
    delegate: id,
}
impl HostObject for FBSessionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation FBSession: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(FBSessionHostObject {
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// Тот самый метод инициализации из твоего лога
+ (id)sessionForApplication:(id)_app_key getSessionProxy:(bool)_proxy delegate:(id)delegate {
    log!("FBSession sessionForApplication:getSessionProxy:delegate: stubbed");

    let session: id = msg![env; this alloc];
    let session: id = msg![env; session init];

    env.objc.borrow_mut::<FBSessionHostObject>(session).delegate = delegate;

    // Сразу же имитируем неудачный логин, чтобы игра не ждала ответа от
    // серверов FB
    if delegate != nil {
        // В старом Facebook Connect SDK для iOS ошибка логина обычно
        // обрабатывалась этим методом
        let sel = env.objc.register_host_selector(
            "sessionDidNotLogin:".to_string(),
            &mut env.mem
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate sessionDidNotLogin:session];
        }
    }

    session
}

- (id)init {
    this
}

- (())dealloc {
    // delegate is weak — no release
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)delegate {
    env.objc.borrow::<FBSessionHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<FBSessionHostObject>(this).delegate = delegate;
}

- (bool)resume {
    log!("FBSession resume: stubbed");
    // Возвращаем false, сообщая игре, что активной сессии нет
    false
}

@end

};
