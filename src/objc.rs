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

use crate::dyld::{export_c_func, FunctionExports};
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
pub use selectors::{selector, SEL};

use classes::{ClassHostObject, FakeClass, UnimplementedClass, CLASS_LISTS};
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

pub const TYPE_ID: char = '@';
pub const TYPE_CLASS: char = '#';
pub const TYPE_SEL: char = ':';
pub const TYPE_CHAR: char = 'c';
pub const TYPE_UCHAR: char = 'C';
pub const TYPE_SHORT: char = 's';
pub const TYPE_USHORT: char = 'S';
pub const TYPE_INT: char = 'i';
pub const TYPE_UINT: char = 'I';
pub const TYPE_LONG: char = 'l';
pub const TYPE_ULONG: char = 'L';
pub const TYPE_LONGLONG: char = 'q';
pub const TYPE_ULONGLONG: char = 'Q';
pub const TYPE_FLOAT: char = 'f';
pub const TYPE_DOUBLE: char = 'd';
#[allow(unused)]
pub const TYPE_BITFIELD: char = 'b';
pub const TYPE_BOOL: char = 'B';
pub const TYPE_VOID: char = 'v';
pub const TYPE_UNDEF: char = '?';
#[allow(unused)]
pub const TYPE_PTR: char = '^';
#[allow(unused)]
pub const TYPE_CHAR_PTR: char = '*';
#[allow(unused)]
pub const TYPE_ARRAY_OPEN: char = '[';
#[allow(unused)]
pub const TYPE_ARRAY_CLOSE: char = ']';
#[allow(unused)]
pub const TYPE_UNION_OPEN: char = '(';
#[allow(unused)]
pub const TYPE_UNION_CLOSE: char = ')';
#[allow(unused)]
pub const TYPE_STRUCT_OPEN: char = '{';
#[allow(unused)]
pub const TYPE_STRUCT_CLOSE: char = '}';

#[allow(unused)]
pub const METHOD_ENCODING_CONST: char = 'r';
#[allow(unused)]
pub const METHOD_ENCODING_IN: char = 'n';
#[allow(unused)]
pub const METHOD_ENCODING_INOUT: char = 'N';
#[allow(unused)]
pub const METHOD_ENCODING_OUT: char = 'o';
#[allow(unused)]
pub const METHOD_ENCODING_BYCOPY: char = 'O';
#[allow(unused)]
pub const METHOD_ENCODING_BYREF: char = 'R';
#[allow(unused)]
pub const METHOD_ENCODING_ONEWAY: char = 'V';

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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(objc_msgSend(_, _)),
    export_c_func!(objc_msgSend_stret(_, _, _)),
    export_c_func!(objc_msgSendSuper2(_, _)),
    export_c_func!(objc_getProperty(_, _, _, _)),
    export_c_func!(objc_setProperty(_, _, _, _, _, _)),
    export_c_func!(objc_copyStruct(_, _, _, _, _)),
    export_c_func!(objc_sync_enter(_)),
    export_c_func!(objc_sync_exit(_)),
    export_c_func!(sel_registerName(_)),
];
