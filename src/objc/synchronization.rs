use std::num::NonZeroU32;

use crate::environment::Environment;

use super::id;

/// Backing function of @synchronized block entry.
/// This function is entirely undocumented, with
/// [source code provided](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/objc-sync.h.auto.html).
pub(super) fn objc_sync_enter(env: &mut Environment, obj: id) -> u32 {
    match env.objc.sync_state.get_mut(&obj) {
        Some(sync_data) if sync_data.0 == env.current_thread => {
            sync_data.1 = sync_data.1.checked_add(1).unwrap();
            log_dbg!("Re-entry of {:?} to synchronized", obj);
        }
        Some(_) => {
            // TODO: block thread here
            unimplemented!("Attempted cross-thread @synchronized of {:?}", obj);
        }
        None => {
            env.objc
                .sync_state
                .insert(obj, (env.current_thread, NonZeroU32::new(1).unwrap()));
            log_dbg!("Added {:?} to synchronized", obj);
        }
    }
    0u32 // OK
}

/// Backing function of @synchronized block exit.
/// This function is entirely undocumented, with
/// [source code provided](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/objc-sync.h.auto.html).
pub(super) fn objc_sync_exit(env: &mut Environment, obj: id) -> u32 {
    match env.objc.sync_state.get_mut(&obj) {
        Some((tid, count)) if *tid == env.current_thread => {
            if count.get() == 1 {
                env.objc.sync_state.remove(&obj);
                log_dbg!("Regular @synchronized block exit for {:?}, unlocked", obj);
            } else {
                *count = NonZeroU32::new(count.get() - 1).unwrap();
                log_dbg!(
                    "Regular @synchronized block exit for {:?}: {} locks remain",
                    obj,
                    count.get()
                );
            }
        }
        Some(_) => {
            panic!(
                "Attempted exit of @synchronized block for {:?} not owned by current thread",
                obj
            );
            // See below.
        }
        None => {
            panic!(
                "Attempt to exit from @synchronized block for {:?} that was not entered properly",
                obj
            );
            // Should return an error (non-zero), although I don't think it's ever checked?
            // Something probably went wrong to get here.
        }
    }

    0u32 // OK
}
