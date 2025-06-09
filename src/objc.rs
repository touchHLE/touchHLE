/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Objective-C runtime.
//!
//! Apple's [Programming with Objective-C](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ProgrammingWithObjectiveC/Introduction/Introduction.html)
//! is a useful introduction to the language from a user's perspective.
//! There are further resources in the child modules of this module, but they
//! are more implementation-specific.
//!
//! The strategy for this emulator will be to provide our own implementations of
//! an Objective-C runtime and libraries for it (Foundation etc). These
//! implementations will be "host code": Rust code forming part of the emulator,
//! not emulated code. The runtime will need to be able to handle classes that
//! originate from the guest app, classes defined by the host, and sometimes
//! classes that are both (considering Objective-C's support for inheritance,
//! categories and dynamic class editing).

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant, HostDylib};
use crate::MutexId;
use std::collections::HashMap;

mod classes;
mod messages;
mod methods;
mod objects;
mod properties;
mod selectors;
mod synchronization;

pub use classes::{objc_classes, Class, ClassExports, ClassTemplate};
pub use messages::{
    autorelease, msg, msg_class, msg_send, msg_send_super2, msg_super, objc_super, release, retain,
};
pub use methods::{HostIMP, IMP};
pub use objects::{
    id, impl_HostObject_with_superclass, nil, AnyHostObject, HostObject, TrivialHostObject,
};
pub use properties::todo_objc_setter;
pub use selectors::{selector, SEL};

use crate::mem::ConstVoidPtr;
use crate::Environment;
use classes::{ClassHostObject, FakeClass, UnimplementedClass};
use messages::{
    objc_msgSend, objc_msgSendSuper2, objc_msgSend_stret, MsgSendSignature, MsgSendSuperSignature,
};
use methods::method_list_t;
use objects::{objc_object, HostObjectEntry};
use properties::{ivar_list_t, objc_copyStruct, objc_getProperty, objc_setProperty};
use selectors::sel_registerName;
use synchronization::{objc_sync_enter, objc_sync_exit};

/// Typedef for `NSZone *`. This is a [fossil type] found in the signature of
/// `allocWithZone:` and similar methods. Its value is always ignored.
///
/// [fossil type]: https://en.wiktionary.org/wiki/fossil_word
pub type NSZonePtr = crate::mem::MutVoidPtr;

/// Main type holding Objective-C runtime state.
pub struct ObjC {
    /// Known selectors (interned method name strings).
    selectors: HashMap<String, SEL>,

    /// Mapping of known (guest) object pointers to their host objects.
    ///
    /// If an object isn't in this map, we will consider it not to exist.
    objects: HashMap<id, HostObjectEntry>,

    /// Known classes.
    ///
    /// Look at the `isa` to get the metaclass for a class.
    classes: HashMap<String, Class>,

    /// Mutexes used in @synchronized blocks (objc_sync_enter/exit).
    sync_mutexes: HashMap<id, MutexId>,

    /// Temporary storage for optional type information when sending a message.
    /// Type information isn't part of the `objc_msgSend` ABI, so an alternative
    /// channel is needed.
    message_type_info: Option<(std::any::TypeId, &'static str)>,
}

impl ObjC {
    pub fn new() -> ObjC {
        ObjC {
            selectors: HashMap::new(),
            objects: HashMap::new(),
            classes: HashMap::new(),
            sync_mutexes: HashMap::new(),
            message_type_info: None,
        }
    }
}

pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libobjc.A.dylib",
    aliases: &["/usr/lib/libobjc.dylib"],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};

const CONSTANTS: ConstantExports = &[
    // We don't use these in our Objective-C runtime, but exporting useless
    // symbols for these silences the warning about the unhandled relocation,
    // and avoids a linker error for the integration tests.
    ("__objc_empty_vtable", HostConstant::NullPtr),
    ("__objc_empty_cache", HostConstant::NullPtr),
];

/// Block support is iOS 4+, but it seems like Block Runtime Helpers
/// could still be called on even if minimal iOS version is set to 3.x?
///
/// ref. <https://clang.llvm.org/docs/Block-ABI-Apple.html#runtime-helper-functions>
fn _Block_object_dispose(_env: &mut Environment, object: ConstVoidPtr, flags: i32) {
    // `BLOCK_FIELD_IS_BYREF` flag defines an on stack structure holding
    // the __block variable. It is _probably_ safe to ignore.
    // TODO: properly implement for block support
    assert!(flags == 8); // BLOCK_FIELD_IS_BYREF
    log!(
        "Warning: Ignoring _Block_object_dispose({:?}, BLOCK_FIELD_IS_BYREF)",
        object
    );
}

const FUNCTIONS: FunctionExports = &[
    export_c_func!(objc_msgSend(_, _)),
    export_c_func!(objc_msgSend_stret(_, _, _)),
    export_c_func!(objc_msgSendSuper2(_, _)),
    export_c_func!(objc_getProperty(_, _, _, _)),
    export_c_func!(objc_setProperty(_, _, _, _, _, _)),
    export_c_func!(objc_copyStruct(_, _, _, _, _)),
    export_c_func!(objc_sync_enter(_)),
    export_c_func!(objc_sync_exit(_)),
    export_c_func!(sel_registerName(_)),
    export_c_func!(_Block_object_dispose(_, _)),
];
