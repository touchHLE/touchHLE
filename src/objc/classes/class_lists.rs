//! Separate module just for the class lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{core_animation, foundation, opengles, uikit};

/// All the lists of classes that the runtime should search through.
pub const CLASS_LISTS: &[super::ClassExports] = &[
    core_animation::ca_eagl_layer::CLASSES,
    core_animation::ca_layer::CLASSES,
    foundation::ns_array::CLASSES,
    foundation::ns_autorelease_pool::CLASSES,
    foundation::ns_bundle::CLASSES,
    foundation::ns_character_set::CLASSES,
    foundation::ns_coder::CLASSES,
    foundation::ns_dictionary::CLASSES,
    foundation::ns_keyed_unarchiver::CLASSES,
    foundation::ns_locale::CLASSES,
    foundation::ns_object::CLASSES,
    foundation::ns_run_loop::CLASSES,
    foundation::ns_string::CLASSES,
    foundation::ns_url::CLASSES,
    foundation::ns_value::CLASSES,
    opengles::eagl::CLASSES,
    uikit::ui_accelerometer::CLASSES,
    uikit::ui_application::CLASSES,
    uikit::ui_nib::CLASSES,
    uikit::ui_responder::CLASSES,
    uikit::ui_screen::CLASSES,
    uikit::ui_view::CLASSES,
    uikit::ui_window::CLASSES,
];
