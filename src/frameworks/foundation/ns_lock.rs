/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::environment::{MutexId, MutexType};
use crate::objc::{id, ClassExports, HostObject, NSZonePtr};
use crate::objc_classes;

struct LockHostObject {
    mutex_id: MutexId,
}
impl HostObject for LockHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSLock: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(LockHostObject {
        mutex_id: env.mutex_state.init_mutex(MutexType::PTHREAD_MUTEX_NORMAL),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

-(())lock {
    let host_object = env.objc.borrow::<LockHostObject>(this);
    env.lock_mutex(host_object.mutex_id).unwrap();
}

-(())unlock {
    let host_object = env.objc.borrow::<LockHostObject>(this);
    env.unlock_mutex(host_object.mutex_id).unwrap();
}

-(())dealloc {
    let host_object = env.objc.borrow_mut::<LockHostObject>(this);
    env.mutex_state.destroy_mutex(host_object.mutex_id).unwrap();

    env.objc.dealloc_object(this, &mut env.mem)
}
@end
};
