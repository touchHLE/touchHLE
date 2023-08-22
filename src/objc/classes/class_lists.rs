/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Separate module just for the class lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{core_animation, core_graphics, foundation, media_player, opengles, uikit};

/// All the lists of classes that the runtime should search through.
pub const CLASS_LISTS: &[super::ClassExports] = &[
    crate::app_picker::CLASSES, // Not a framework! Special internal classes.
    core_animation::ca_eagl_layer::CLASSES,
    core_animation::ca_layer::CLASSES,
    core_graphics::cg_data_provider::CLASSES,
    core_graphics::cg_color_space::CLASSES,
    core_graphics::cg_context::CLASSES,
    core_graphics::cg_image::CLASSES,
    foundation::ns_array::CLASSES,
    foundation::ns_autorelease_pool::CLASSES,
    foundation::ns_bundle::CLASSES,
    foundation::ns_character_set::CLASSES,
    foundation::ns_coder::CLASSES,
    foundation::ns_data::CLASSES,
    foundation::ns_date::CLASSES,
    foundation::ns_dictionary::CLASSES,
    foundation::ns_enumerator::CLASSES,
    foundation::ns_file_manager::CLASSES,
    foundation::ns_keyed_unarchiver::CLASSES,
    foundation::ns_locale::CLASSES,
    foundation::ns_notification::CLASSES,
    foundation::ns_notification_center::CLASSES,
    foundation::ns_null::CLASSES,
    foundation::ns_object::CLASSES,
    foundation::ns_process_info::CLASSES,
    foundation::ns_run_loop::CLASSES,
    foundation::ns_set::CLASSES,
    foundation::ns_string::CLASSES,
    foundation::ns_thread::CLASSES,
    foundation::ns_timer::CLASSES,
    foundation::ns_url::CLASSES,
    foundation::ns_user_defaults::CLASSES,
    foundation::ns_value::CLASSES,
    media_player::movie_player::CLASSES,
    opengles::eagl::CLASSES,
    uikit::ui_accelerometer::CLASSES,
    uikit::ui_application::CLASSES,
    uikit::ui_color::CLASSES,
    uikit::ui_device::CLASSES,
    uikit::ui_event::CLASSES,
    uikit::ui_font::CLASSES,
    uikit::ui_image::CLASSES,
    uikit::ui_image_picker_controller::CLASSES,
    uikit::ui_nib::CLASSES,
    uikit::ui_responder::CLASSES,
    uikit::ui_screen::CLASSES,
    uikit::ui_touch::CLASSES,
    uikit::ui_view::CLASSES,
    uikit::ui_view::ui_alert_view::CLASSES,
    uikit::ui_view::ui_control::CLASSES,
    uikit::ui_view::ui_control::ui_button::CLASSES,
    uikit::ui_view::ui_control::ui_text_field::CLASSES,
    uikit::ui_view::ui_image_view::CLASSES,
    uikit::ui_view::ui_label::CLASSES,
    uikit::ui_view::ui_window::CLASSES,
    uikit::ui_view_controller::CLASSES,
];
