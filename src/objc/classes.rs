//! Handling of Objective-C classes and metaclasses.
//!
//! Note that metaclasses are just a special case of classes.
//!
//! Resources:
//! - [[objc explain]: Classes and metaclasses](http://www.sealiesoftware.com/blog/archive/2009/04/14/objc_explain_Classes_and_metaclasses.html), especially [the PDF diagram](http://www.sealiesoftware.com/blog/class%20diagram.pdf)

use super::{id, nil, HostObject};
use crate::memory::Memory;

/// Generic pointer to an Objective-C class or metaclass.
///
/// The name is standard Objective-C.
///
/// Apple's runtime has a `objc_class` definition similar to `objc_object`.
/// We could do the same thing here, but it doesn't seem worth it, as we can't
/// get the same unidirectional type safety.
pub type Class = id;

/// Placeholder object for classes and metaclasses referenced by the app that
/// we don't have an implementation for.
///
/// This lets us delay errors about missing implementations until the first
/// time the app actually uses them (e.g. when a message is sent).
struct UnimplementedClass {
    name: String,
    is_metaclass: bool,
}

impl HostObject for UnimplementedClass {
    fn is_unimplemented_class(&self) -> Option<(&str, bool)> {
        Some((&self.name, self.is_metaclass))
    }
}

impl super::ObjC {
    /// For use by [crate::dyld]: get the class referenced by an external
    /// relocation in the application.
    pub fn link_class(&mut self, name: &str, is_metaclass: bool, mem: &mut Memory) -> Class {
        if let Some(&class) = self.classes.get(name) {
            if is_metaclass {
                return Self::read_isa(class, mem);
            } else {
                return class;
            }
        };

        // TODO: Look up host implementations and link those, where available

        let metaclass_host_object = Box::new(UnimplementedClass {
            name: name.to_string(),
            is_metaclass: true,
        });
        // the metaclass's isa can't be nil, so it should point back to the
        // metaclass, but we can't make the object self-referential in a single
        // step, so: write nil and then overwrite it.
        let metaclass = self.alloc_object(nil, metaclass_host_object, mem);
        Self::write_isa(metaclass, metaclass, mem);

        let class_host_object = Box::new(UnimplementedClass {
            name: name.to_string(),
            is_metaclass: true,
        });
        let class = self.alloc_object(metaclass, class_host_object, mem);

        self.classes.insert(name.to_string(), class);

        if is_metaclass {
            metaclass
        } else {
            class
        }
    }
}
