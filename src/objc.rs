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

use std::collections::HashMap;

mod classes;
mod messages;
mod methods;
mod objects;
mod selectors;

pub use classes::{objc_classes, Class, ClassExports, ClassTemplate};
pub use messages::{msg, msg_class, msg_send};
pub use methods::{HostIMP, IMP};
pub use objects::{id, nil, AnyHostObject, HostObject, TrivialHostObject};
pub use selectors::{selector, SEL};

use classes::{ClassHostObject, UnimplementedClass, CLASS_LISTS};
use messages::objc_msgSend;

/// Main type holding Objective-C runtime state.
pub struct ObjC {
    /// Known selectors (interned method name strings).
    selectors: HashMap<String, SEL>,

    /// Mapping of known (guest) object pointers to their host objects.
    ///
    /// If an object isn't in this map, we will consider it not to exist.
    objects: HashMap<id, Box<dyn AnyHostObject>>,

    /// Known classes.
    ///
    /// Look at the `isa` to get the metaclass for a class.
    classes: HashMap<String, Class>,
}

impl ObjC {
    pub fn new() -> ObjC {
        ObjC {
            selectors: HashMap::new(),
            objects: HashMap::new(),
            classes: HashMap::new(),
        }
    }
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(objc_msgSend(_, _))];
