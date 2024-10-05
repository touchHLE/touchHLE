/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSNotificationCenter`.

use super::ns_notification::NSNotificationName;
use super::ns_string;

use crate::objc::{
    id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, SEL,
};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    default_center: Option<id>,
}

#[derive(Clone)]
struct Observer {
    observer: id,
    selector: SEL,
    object: id,
}

struct NSNotificationCenterHostObject {
    observers: HashMap<Cow<'static, str>, Vec<Observer>>,
}
impl HostObject for NSNotificationCenterHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSNotificationCenter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSNotificationCenterHostObject {
        observers: HashMap::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)defaultCenter {
    if let Some(c) = env.framework_state.foundation.ns_notification_center.default_center {
        c
    } else {
        let new: id = msg![env; this new];
        env.framework_state.foundation.ns_notification_center.default_center = Some(new);
        new
    }
}

- (())dealloc {
    let host_obj = env.objc.borrow_mut::<NSNotificationCenterHostObject>(this);
    let observers = std::mem::take(&mut host_obj.observers);
    for observer in observers.values().flatten() {
        release(env, observer.observer);
        release(env, observer.object);
    }
    env.objc.dealloc_object(this, &mut env.mem);
}

- (())addObserver:(id)observer
         selector:(SEL)selector
             name:(NSNotificationName)name
           object:(id)object {
    if name == nil &&
        env.bundle.bundle_identifier().starts_with("com.chillingo.cuttherope") &&
        selector == env.objc.lookup_selector("fetchUpdateNotification:").unwrap() {
        // As we nullified Flurry SDK, we also need to no-op
        // related notifications
        log!("Applying game-specific hack for Cut the Rope: ignoring addObserver:selector:name:object: for fetchUpdateNotification:");
        return;
    }
    // TODO: handle case where name is nil
    // Usually a static string, so no real copy will happen
    let name = ns_string::to_rust_string(env, name);

    log_dbg!(
        "[(NSNotificationCenter*){:?} addObserver:{:?} selector:{:?} name:{:?} object:{:?}",
        this,
        observer,
        selector,
        name,
        object,
    );

    retain(env, observer);
    retain(env, object); // TODO: is it correct that this is retained?

    let host_obj = env.objc.borrow_mut::<NSNotificationCenterHostObject>(this);
    host_obj.observers.entry(name).or_default().push(Observer {
        observer,
        selector,
        object,
    });
}

- (())removeObserver:(id)observer
                name:(NSNotificationName)name
              object:(id)object {
    assert!(observer != nil); // TODO

    // TODO: handle case where name is nil
    // Usually a static string, so no real copy will happen
    let name = ns_string::to_rust_string(env, name);

    log_dbg!(
        "[(NSNotificationCenter*){:?} removeObserver:{:?} name:{:?} object:{:?}",
        this,
        observer,
        name,
        object,
    );

    let host_obj = env.objc.borrow_mut::<NSNotificationCenterHostObject>(this);
    let Some(observers) = host_obj.observers.get_mut(&name) else {
        return;
    };

    // TODO: is this the correct behaviour, can an observer be registered
    // several times?
    let mut removed_observers = Vec::new();

    let mut i = 0;
    while i < observers.len() {
        if observers[i].observer == observer && (object == nil || object == observers[i].object) {
            removed_observers.push(observers.swap_remove(i));
        } else {
            i += 1;
        }
    }

    for removed_observer in removed_observers {
        release(env, removed_observer.observer);
        release(env, removed_observer.object);
    }
}

- (())postNotification:(id)notification {
    log_dbg!(
        "[(NSNotificationCenter*){:?} postNotification:{:?}]",
        this,
        notification,
    );

    let name: id = msg![env; notification name];
    // Usually a static string, so no real copy will happen
    let name = ns_string::to_rust_string(env, name);

    let notification_poster: id = msg![env; notification object];

    log_dbg!("Notification is a {:?} posted by {:?}", name, notification_poster);

    let host_obj = env.objc.borrow_mut::<NSNotificationCenterHostObject>(this);
    let Some(observers) = host_obj.observers.get(&name).cloned() else {
        return;
    };
    for Observer { observer, selector, object } in observers {
        // The object argument is a filter for which notification sources the
        // observer is interested in.
        if object != nil && notification_poster != object {
            continue;
        }

        log_dbg!(
            "Notification {:?} observed, sending {:?} message to {:?}",
            notification,
            selector.as_str(&env.mem),
            observer
        );

        // In some cases, observer could be removed during the
        // processing of the notification, effectively releasing it.
        // (This is happening with Spore Origins)
        // We need to retain it for correctness.
        retain(env, observer);
        // Signature should be `- (void)notification:(NSNotification *)notif`.
        let _: () = msg_send(env, (observer, selector, notification));
        release(env, observer);
    }
}
- (())postNotificationName:(NSNotificationName)name
                    object:(id)object {
    msg![env; this postNotificationName:name
                                 object:object
                               userInfo:nil]
}
- (())postNotificationName:(NSNotificationName)name
                    object:(id)object
                  userInfo:(id)user_info { // NSDictionary*
    let notification: id = msg_class![env; NSNotification alloc];
    let notification: id = msg![env; notification initWithName:name
                                                        object:object
                                                      userInfo:user_info];
    let _: () = msg![env; this postNotification:notification];
    release(env, notification);
}

@end

};
