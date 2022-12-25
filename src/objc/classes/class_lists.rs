//! Separate module just for the class lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{foundation, uikit};

/// All the lists of classes that the runtime should search through.
pub const CLASS_LISTS: &[super::ClassExports] = &[
    foundation::ns_array::CLASSES,
    foundation::ns_autorelease_pool::CLASSES,
    foundation::ns_coder::CLASSES,
    foundation::ns_keyed_unarchiver::CLASSES,
    foundation::ns_object::CLASSES,
    foundation::ns_string::CLASSES,
    uikit::ui_application::CLASSES,
    uikit::ui_nib::CLASSES,
    uikit::ui_responder::CLASSES,
    uikit::ui_view::CLASSES,
    uikit::ui_window::CLASSES,
];
