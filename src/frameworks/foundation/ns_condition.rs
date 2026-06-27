/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCondition` and `NSConditionLock`.

use std::collections::VecDeque;

use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// =========================================================================
// MARK: - NSCondition host object
// =========================================================================

#[derive(Default)]
struct NSConditionHostObject {
    /// Name NSString* — for debugging.
    name: id,
    /// Состояние внутреннего мьютекса
    locked: bool,
    /// Потоки, ожидающие захвата блокировки (lock)
    lock_waiting_threads: VecDeque<crate::environment::ThreadId>,
    /// Потоки, ожидающие сигнала (wait)
    waiting_threads: VecDeque<crate::environment::ThreadId>,
}
impl HostObject for NSConditionHostObject {}

// =========================================================================
// MARK: - NSConditionLock host object
// =========================================================================

#[derive(Default)]
struct NSConditionLockHostObject {
    name: id,
    /// The current condition value.
    condition: NSInteger,
    /// Whether the lock is currently held.
    locked: bool,
    /// Потоки, ожидающие захвата. Option<NSInteger> указывает, ждет ли поток
    /// конкретного состояния (Some) или просто освобождения блокировки (None).
    waiting_threads: VecDeque<(crate::environment::ThreadId, Option<NSInteger>)>,
}
impl HostObject for NSConditionLockHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - NSCondition
// =========================================================================

@implementation NSCondition: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSConditionHostObject {
        name: nil,
        locked: false,
        lock_waiting_threads: VecDeque::new(),
        waiting_threads: VecDeque::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    let name = env.objc.borrow::<NSConditionHostObject>(this).name;
    release(env, name);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Name

- (id)name { // NSString*
    env.objc.borrow::<NSConditionHostObject>(this).name
}

- (())setName:(id)name { // NSString*
    let old = env.objc.borrow::<NSConditionHostObject>(this).name;
    release(env, old);
    retain(env, name);
    env.objc.borrow_mut::<NSConditionHostObject>(this).name = name;
}

// MARK: NSLocking protocol

- (())lock {
    loop {
        {
            let host = env.objc.borrow_mut::<NSConditionHostObject>(this);
            if !host.locked {
                host.locked = true;
                break;
            }
            // Мьютекс занят, добавляем себя в очередь и засыпаем
            let current_thread = env.current_thread;
            host.lock_waiting_threads.push_back(current_thread);
        }
        env.suspend_thread(env.current_thread);
    }
}

- (())unlock {
    let host = env.objc.borrow_mut::<NSConditionHostObject>(this);
    host.locked = false;

    // Будим первый поток в очереди
    if let Some(thread) = host.lock_waiting_threads.pop_front() {
        env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
    }
}

// MARK: Wait / signal / broadcast

- (())wait {
    // 1. Атомарно освобождаем блокировку и добавляем себя в очередь ожидания
    // сигнала
    {
        let host = env.objc.borrow_mut::<NSConditionHostObject>(this);
        host.locked = false;
        if let Some(thread) = host.lock_waiting_threads.pop_front() {
            env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
        }

        let current_thread = env.current_thread;
        host.waiting_threads.push_back(current_thread);
    }

    // 2. Засыпаем, пока нас не разбудит `signal` или `broadcast`
    env.suspend_thread(env.current_thread);

    // 3. По правилам POSIX, после пробуждения необходимо снова захватить
    // мьютекс
    () = msg![env; this lock];
}

- (bool)waitUntilDate:(id)limit_date { // NSDate*
    let ti: f64 = msg![env; limit_date timeIntervalSinceNow];
    if ti <= 0.0 {
        return false;
    }

    // 1. Освобождаем блокировку
    {
        let host = env.objc.borrow_mut::<NSConditionHostObject>(this);
        host.locked = false;
        if let Some(thread) = host.lock_waiting_threads.pop_front() {
            env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
        }

        let current_thread = env.current_thread;
        host.waiting_threads.push_back(current_thread);
    }

    // 2. Засыпаем с таймаутом
    let until = std::time::Instant::now()
        + super::ns_time_interval_to_duration_or_zero(ti);
    env.yield_thread(crate::environment::ThreadBlock::Sleeping(until));

    // 3. Проверяем, проснулись ли мы сами (таймаут) или нас разбудили
    let mut timed_out = false;
    {
        let host = env.objc.borrow_mut::<NSConditionHostObject>(this);
        let current_thread = env.current_thread;
        if let Some(pos) = host.waiting_threads.iter().position(|&t| t == current_thread) {
            host.waiting_threads.remove(pos);
            timed_out = true;
        }
    }

    // 4. Захватываем блокировку обратно
    () = msg![env; this lock];

    !timed_out
}

- (())signal {
    // Будим один ждущий поток
    if let Some(thread) = env.objc.borrow_mut::<NSConditionHostObject>(this).waiting_threads.pop_front() {
        env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
    }
}

- (())broadcast {
    // Будим все ждущие потоки
    let threads = std::mem::take(&mut env.objc.borrow_mut::<NSConditionHostObject>(this).waiting_threads);
    for thread in threads {
        env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
    }
}

// MARK: Description

- (id)description {
    let (name, waiting_count) = {
        let h = env.objc.borrow::<NSConditionHostObject>(this);
        (h.name, h.waiting_threads.len())
    };
    let name_str = if name != nil {
        crate::frameworks::foundation::ns_string::to_rust_string(env, name).into_owned()
    } else {
        "(nil)".to_string()
    };
    let s = format!(
        "<NSCondition: {:?}; name={}; waitingThreads={}>",
        this, name_str, waiting_count
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - NSConditionLock
// =========================================================================

@implementation NSConditionLock: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSConditionLockHostObject {
        name: nil,
        condition: 0,
        locked: false,
        waiting_threads: VecDeque::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (id)initWithCondition:(NSInteger)condition {
    env.objc.borrow_mut::<NSConditionLockHostObject>(this).condition = condition;
    this
}

- (())dealloc {
    let name = env.objc.borrow::<NSConditionLockHostObject>(this).name;
    release(env, name);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Name

- (id)name {
    env.objc.borrow::<NSConditionLockHostObject>(this).name
}

- (())setName:(id)name {
    let old = env.objc.borrow::<NSConditionLockHostObject>(this).name;
    release(env, old);
    retain(env, name);
    env.objc.borrow_mut::<NSConditionLockHostObject>(this).name = name;
}

// MARK: Condition

- (NSInteger)condition {
    env.objc.borrow::<NSConditionLockHostObject>(this).condition
}

// MARK: NSLocking protocol

- (())lock {
    loop {
        {
            let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
            if !host.locked {
                host.locked = true;
                break;
            }
            let current_thread = env.current_thread;
            host.waiting_threads.push_back((current_thread, None));
        }
        env.suspend_thread(env.current_thread);
    }
}

- (())unlock {
    let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
    host.locked = false;

    // Ищем первый поток, который ждет освобождения или ждет текущего condition
    let mut to_wake = None;
    for (i, &(_, cond)) in host.waiting_threads.iter().enumerate() {
        if cond.is_none() || cond == Some(host.condition) {
            to_wake = Some(i);
            break;
        }
    }

    if let Some(i) = to_wake {
        let (thread, _) = host.waiting_threads.remove(i).unwrap();
        env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
    }
}

// MARK: Conditional locking

- (())lockWhenCondition:(NSInteger)condition {
    loop {
        {
            let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
            if !host.locked && host.condition == condition {
                host.locked = true;
                break;
            }
            let current_thread = env.current_thread;
            host.waiting_threads.push_back((current_thread, Some(condition)));
        }
        env.suspend_thread(env.current_thread);
    }
}

- (bool)lockWhenCondition:(NSInteger)condition
               beforeDate:(id)limit_date { // NSDate*
    loop {
        {
            let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
            if !host.locked && host.condition == condition {
                host.locked = true;
                return true;
            }
        }

        let ti: f64 = msg![env; limit_date timeIntervalSinceNow];
        if ti <= 0.0 {
            return false;
        }

        {
            let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
            let current_thread = env.current_thread;
            host.waiting_threads.push_back((current_thread, Some(condition)));
        }

        let until = std::time::Instant::now()
            + super::ns_time_interval_to_duration_or_zero(ti);
        env.yield_thread(crate::environment::ThreadBlock::Sleeping(until));

        // Удаляем себя из очереди, если мы проснулись по таймауту
        // (если разбудили — следующая итерация захватит блокировку)
        {
            let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
            let current_thread = env.current_thread;
            if let Some(pos) = host.waiting_threads.iter().position(|&(t, _)| t == current_thread) {
                host.waiting_threads.remove(pos);
            }
        }
    }
}

- (bool)tryLock {
    let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
    if !host.locked {
        host.locked = true;
        return true;
    }
    false
}

- (bool)tryLockWhenCondition:(NSInteger)condition {
    let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
    if !host.locked && host.condition == condition {
        host.locked = true;
        return true;
    }
    false
}

- (())unlockWithCondition:(NSInteger)condition {
    let host = env.objc.borrow_mut::<NSConditionLockHostObject>(this);
    host.locked = false;
    host.condition = condition;

    // Ищем первый поток, который ждет нового condition
    let mut to_wake = None;
    for (i, &(_, cond)) in host.waiting_threads.iter().enumerate() {
        if cond.is_none() || cond == Some(condition) {
            to_wake = Some(i);
            break;
        }
    }

    if let Some(i) = to_wake {
        let (thread, _) = host.waiting_threads.remove(i).unwrap();
        env.threads[thread].blocked_by = crate::environment::ThreadBlock::NotBlocked;
    }
}

// MARK: Description

- (id)description {
    let (condition, locked) = {
        let h = env.objc.borrow::<NSConditionLockHostObject>(this);
        (h.condition, h.locked)
    };
    let s = format!(
        "<NSConditionLock: {:?}; condition={}; locked={}>",
        this, condition, locked
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

};
