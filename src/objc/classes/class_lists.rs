//! Separate module just for the class lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::foundation;

/// All the lists of classes that the runtime should search through.
pub const CLASS_LISTS: &[super::ClassExports] = &[
    foundation::ns_autorelease_pool::CLASSES,
    foundation::ns_object::CLASSES,
];
